//! Lightweight static checks surfaced inline in the editor.
//!
//! Vulpin evaluates an undefined `$name` to `None` *silently* (`Vulpin/src/vm.c`,
//! `eval_expr` / `ND_IDENT`: no entry in `vm->vars` → `val_none()`, no error,
//! exit 0). A typo'd variable therefore prints `None` instead of failing. This
//! flags every `$name` whose `name` is never defined anywhere in the buffer.
//!
//! The scan is deliberately flat (a name defined in *any* scope counts as
//! defined): Vulpin functions see globals via `vextend`, and a flat set keeps
//! false positives — the annoying kind — to a minimum.

use std::collections::HashSet;

use crate::buffer::{Buffer, Position};

/// Modules usable without a `U` import (`Vulpin/src/vm.c` `builtin_call`).
const BUILTIN_MODULES: &[&str] = &["math", "os", "random"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub start: Position,
    pub end: Position,
    pub message: String,
}

/// Every `$name` reference in the buffer whose `name` is never defined.
pub fn check(buf: &Buffer) -> Vec<Diagnostic> {
    let mut defined: HashSet<String> = BUILTIN_MODULES.iter().map(|s| s.to_string()).collect();
    let lines: Vec<String> = (0..buf.line_count()).map(|i| buf.line_text(i)).collect();

    for line in &lines {
        collect_definitions(line, &mut defined);
    }

    let mut out = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '$' && chars.get(i + 1).is_some_and(|c| is_ident_start(*c)) {
                let name_start = i + 1;
                let mut end = name_start;
                while end < chars.len() && is_ident_char(chars[end]) {
                    end += 1;
                }
                let name: String = chars[name_start..end].iter().collect();
                // `$mod.member` — leave module access alone.
                let is_member = chars.get(end) == Some(&'.');
                if !is_member && !defined.contains(&name) {
                    out.push(Diagnostic {
                        start: Position {
                            line: row,
                            col: name_start,
                        },
                        end: Position {
                            line: row,
                            col: end,
                        },
                        message: format!("`{name}` is never assigned — Vulpin reads it as None"),
                    });
                }
                i = end;
            } else if chars[i] == '#' {
                break; // rest of the line is a comment
            } else {
                i += 1;
            }
        }
    }
    out
}

/// Names this line brings into scope.
fn collect_definitions(line: &str, out: &mut HashSet<String>) {
    let trimmed = line.trim_start();
    let mut cs = trimmed.chars();
    let Some(cmd) = cs.next() else { return };
    let rest = cs.as_str().trim();

    match cmd {
        // `K name ...`, `O i start end`, `C err`
        'K' | 'O' | 'C' => {
            if let Some(name) = first_ident(rest) {
                out.insert(name);
            }
        }
        // `F name(a, b)` — the function name and each parameter
        'F' => {
            let mut it = rest.splitn(2, '(');
            if let Some(name) = first_ident(it.next().unwrap_or("")) {
                out.insert(name);
            }
            if let Some(params) = it.next() {
                for p in params.trim_end_matches(')').split(',') {
                    if let Some(name) = first_ident(p) {
                        out.insert(name);
                    }
                }
            }
        }
        // `U mod` or `U "mod/path"` — the imported module's base name
        'U' => {
            let raw = rest.trim_matches('"');
            let base = raw
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(raw)
                .trim_end_matches(".vul");
            if let Some(name) = first_ident(base) {
                out.insert(name);
            }
        }
        // default: a leading `name = ...` assignment (not `==`)
        c if is_ident_start(c) => {
            let name: String = std::iter::once(c)
                .chain(
                    trimmed[c.len_utf8()..]
                        .chars()
                        .take_while(|c| is_ident_char(*c)),
                )
                .collect();
            let after = trimmed[name.len()..].trim_start();
            if after.starts_with('=') && !after.starts_with("==") {
                out.insert(name);
            }
        }
        _ => {}
    }
}

fn first_ident(s: &str) -> Option<String> {
    let s = s.trim_start();
    let mut chars = s.chars();
    let first = chars.next()?;
    if !is_ident_start(first) {
        return None;
    }
    Some(
        std::iter::once(first)
            .chain(chars.take_while(|c| is_ident_char(*c)))
            .collect(),
    )
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(buf: &str) -> Vec<String> {
        check(&Buffer::from_str(buf))
            .into_iter()
            .map(|d| {
                let s = d.start;
                format!("{}:{}", s.line, s.col)
            })
            .collect()
    }

    #[test]
    fn flags_only_undefined_refs() {
        // line 2 has a typo: `$naem` for `$name`
        let src = "name = \"sam\"\nG $name\nG $naem\n";
        let d = check(&Buffer::from_str(src));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].start, Position { line: 2, col: 3 });
        assert_eq!(d[0].end, Position { line: 2, col: 7 });
        assert!(d[0].message.contains("None"));
    }

    #[test]
    fn assignment_arith_input_loop_and_params_all_count() {
        let src = "\
total = 0
K guess \"pick: \" I
O i 1 10
  A total + $i
&
F add(a, b)
  R $a + $b
~
G $add($total, $guess)";
        assert!(check(&Buffer::from_str(src)).is_empty(), "{:?}", names(src));
    }

    #[test]
    fn builtin_and_imported_modules_are_ok() {
        let src = "U math\nU \"lib/helpers.vul\"\nG $math.pi\nG $helpers.thing\n";
        assert!(check(&Buffer::from_str(src)).is_empty());
    }

    #[test]
    fn ignores_dollar_in_comments_and_strings_are_not_special() {
        // `#` starts a comment; `$ghost` after it must not be flagged.
        let src = "x = 1\nG $x  # $ghost here\n";
        assert!(check(&Buffer::from_str(src)).is_empty());
    }

    #[test]
    fn catch_binding_is_defined() {
        let src = "T\n  X \"boom\"\nC err\n  G $err\nY\n";
        assert!(check(&Buffer::from_str(src)).is_empty());
    }
}
