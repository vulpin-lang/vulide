//! Shared line-oriented matching for the in-file find bar (`search.rs`) and
//! project-wide search (`ui/project_search.rs`) — same substring/regex/case
//! semantics in one place, so the two features can't quietly drift apart.

use regex::{Regex, RegexBuilder};

pub enum Matcher {
    Substring {
        needle: Vec<char>,
        case_sensitive: bool,
    },
    Regex(Regex),
}

impl Matcher {
    /// Compile `query` as a literal substring, or — if `regex` is set — as a
    /// regular expression. `Err` carries the compiler's message, for callers
    /// to show in place of "no matches".
    pub fn compile(query: &str, case_sensitive: bool, regex: bool) -> Result<Self, String> {
        if regex {
            RegexBuilder::new(query)
                .case_insensitive(!case_sensitive)
                .build()
                .map(Matcher::Regex)
                .map_err(|e| e.to_string())
        } else {
            Ok(Matcher::Substring {
                needle: query.chars().collect(),
                case_sensitive,
            })
        }
    }

    /// Every non-overlapping match in `line`, as `(start, end)` **char**
    /// indices (not bytes — a match after a non-ASCII glyph would otherwise
    /// misalign the column). Zero-width regex matches (e.g. `a*` matching
    /// nothing) are skipped: there's nothing to select or replace.
    pub fn find_in_line(&self, line: &str) -> Vec<(usize, usize)> {
        match self {
            Matcher::Substring {
                needle,
                case_sensitive,
            } => {
                if needle.is_empty() {
                    return Vec::new();
                }
                let chars: Vec<char> = line.chars().collect();
                let eq = |a: char, b: char| {
                    if *case_sensitive {
                        a == b
                    } else {
                        a.eq_ignore_ascii_case(&b)
                    }
                };
                let mut out = Vec::new();
                let mut i = 0;
                while i + needle.len() <= chars.len() {
                    if chars[i..i + needle.len()]
                        .iter()
                        .zip(needle)
                        .all(|(&a, &b)| eq(a, b))
                    {
                        out.push((i, i + needle.len()));
                        i += needle.len();
                    } else {
                        i += 1;
                    }
                }
                out
            }
            Matcher::Regex(re) => re
                .find_iter(line)
                .filter(|m| !m.as_str().is_empty())
                .map(|m| {
                    let start = line[..m.start()].chars().count();
                    let end = line[..m.end()].chars().count();
                    (start, end)
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substring_is_case_insensitive_by_default() {
        let m = Matcher::compile("foo", false, false).unwrap();
        assert_eq!(m.find_in_line("Foo foo FOO").len(), 3);
    }

    #[test]
    fn substring_case_sensitive() {
        let m = Matcher::compile("foo", true, false).unwrap();
        assert_eq!(m.find_in_line("Foo foo FOO").len(), 1);
    }

    #[test]
    fn substring_is_non_overlapping() {
        let m = Matcher::compile("aa", false, false).unwrap();
        assert_eq!(m.find_in_line("aaaa").len(), 2);
    }

    #[test]
    fn regex_finds_pattern_matches() {
        let m = Matcher::compile(r"\d+", false, true).unwrap();
        assert_eq!(m.find_in_line("a12 b345"), vec![(1, 3), (5, 8)]);
    }

    #[test]
    fn regex_respects_case_sensitivity() {
        let ci = Matcher::compile("foo", false, true).unwrap();
        assert_eq!(ci.find_in_line("FOO").len(), 1);
        let cs = Matcher::compile("foo", true, true).unwrap();
        assert_eq!(cs.find_in_line("FOO").len(), 0);
    }

    #[test]
    fn regex_skips_zero_width_matches() {
        let m = Matcher::compile("x*", false, true).unwrap();
        // "x*" also matches empty at every non-x position — those are dropped.
        assert_eq!(m.find_in_line("axxb"), vec![(1, 3)]);
    }

    #[test]
    fn invalid_regex_is_an_error() {
        assert!(Matcher::compile("(unclosed", false, true).is_err());
    }

    #[test]
    fn regex_columns_survive_non_ascii() {
        let m = Matcher::compile("sumé", false, true).unwrap();
        assert_eq!(m.find_in_line("café résumé"), vec![(7, 11)]);
    }
}
