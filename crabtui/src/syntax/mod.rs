//! Syntax tokenizing.
//!
//! Tokenizers are **single-line**: no cross-line state. That is exact for
//! Vulpin (a line-oriented language) and good enough for the others — a block
//! comment (`/* … */`) that spans lines is only coloured on the lines where it
//! opens or closes. The Python IDE's highlighter is line-based too.

pub mod generic;
pub mod vulpin;

use std::path::Path;

/// A token's character range is a half-open `[start, end)` interval of
/// **character** indices within its line (not bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Comment,
    String,
    Number,
    Variable,
    Function,
    /// Vulpin's leading statement letter.
    Command,
    /// Vulpin's control-flow single char.
    Control,
    /// A reserved word in a conventional language (`if`, `fn`, `def`, …).
    Keyword,
    Operator,
    Bracket,
    Text,
}

/// Which grammar an open buffer is highlighted with. Picked from the file
/// extension; a new/untitled buffer is Vulpin (this is a Vulpin IDE).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Vulpin,
    Python,
    Rust,
    C,
    /// Anything we don't have a grammar for — rendered without highlighting.
    Plain,
}

impl Language {
    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("vul") => Language::Vulpin,
            Some("py" | "pyw" | "pyi") => Language::Python,
            Some("rs") => Language::Rust,
            Some("c" | "h") => Language::C,
            _ => Language::Plain,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Language::Vulpin => "Vulpin",
            Language::Python => "Python",
            Language::Rust => "Rust",
            Language::C => "C",
            Language::Plain => "Plain",
        }
    }

    /// Tokens for a single line, ordered by `start`, non-overlapping. Gaps
    /// between tokens render as `TokenKind::Text`.
    pub fn tokenize_line(self, line: &str) -> Vec<Token> {
        match self {
            Language::Vulpin => vulpin::tokenize(line),
            Language::Python => generic::tokenize(line, &generic::PYTHON),
            Language::Rust => generic::tokenize(line, &generic::RUST),
            Language::C => generic::tokenize(line, &generic::C),
            Language::Plain => Vec::new(),
        }
    }
}
