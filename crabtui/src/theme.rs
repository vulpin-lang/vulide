//! Colour roles — the full ~40 from the Python `THEME_KEYS`.
//!
//! Built-ins are `themes/*.toml`, embedded at compile time so the binary needs
//! no data files. `Theme::parse` also loads user themes (Phase 3 config).
//! Anything a theme file omits falls back to Mocha.

use std::collections::HashMap;

use anyhow::{Context, Result};
use ratatui::style::Color;

use crate::syntax::TokenKind;

/// One field per key in `THEME_KEYS`, same order.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,

    pub bg: Color,
    pub fg: Color,
    pub gutter_bg: Color,
    pub gutter_border: Color,
    pub line_fg: Color,
    pub line_hl: Color,
    pub sel: Color,
    pub current_line: Color,
    pub match_bracket: Color,
    pub comment: Color,
    pub string: Color,
    pub number: Color,
    pub keyword: Color,
    pub command: Color,
    pub control: Color,
    pub variable: Color,
    pub operator: Color,
    pub function: Color,
    pub bracket: Color,
    pub builtin: Color,
    pub toolbar_bg: Color,
    pub toolbar_fg: Color,
    pub tab_bg: Color,
    pub tab_active: Color,
    pub output_bg: Color,
    pub output_fg: Color,
    pub output_err: Color,
    pub output_ok: Color,
    pub dock_bg: Color,
    pub dock_fg: Color,
    pub menu_bg: Color,
    pub menu_fg: Color,
    pub statusbar_bg: Color,
    pub statusbar_fg: Color,
    pub scrollbar: Color,
    pub scrollbar_hover: Color,
    pub autocomplete_bg: Color,
    pub autocomplete_fg: Color,
    pub autocomplete_sel: Color,
    pub accent: Color,
}

const BUILTINS: &[&str] = &[
    include_str!("../themes/mocha.toml"),
    include_str!("../themes/latte.toml"),
    include_str!("../themes/dracula.toml"),
    include_str!("../themes/nord.toml"),
    include_str!("../themes/solarized-dark.toml"),
    include_str!("../themes/monokai.toml"),
];

impl Theme {
    /// The six bundled themes, in menu order.
    pub fn builtins() -> Vec<Theme> {
        BUILTINS
            .iter()
            .map(|s| Theme::parse(s).expect("bundled theme is valid"))
            .collect()
    }

    pub fn mocha() -> Theme {
        // mocha.toml has every key, so the `black` base is never consulted.
        Theme::parse_with_base(BUILTINS[0], &black()).expect("bundled theme is valid")
    }

    /// Parse a theme TOML document. Missing keys fall back to Mocha's value.
    pub fn parse(src: &str) -> Result<Theme> {
        Theme::parse_with_base(src, &Theme::mocha())
    }

    fn parse_with_base(src: &str, base: &Theme) -> Result<Theme> {
        let table: toml::Table = src.parse().context("theme is not valid TOML")?;
        let raw: HashMap<&str, &str> = table
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.as_str(), s)))
            .collect();

        let name = raw
            .get("name")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Custom".into());
        let color = |key: &str, fallback: Color| -> Color {
            raw.get(key).and_then(|s| parse_hex(s)).unwrap_or(fallback)
        };

        Ok(Theme {
            name,
            bg: color("bg", base.bg),
            fg: color("fg", base.fg),
            gutter_bg: color("gutter_bg", base.gutter_bg),
            gutter_border: color("gutter_border", base.gutter_border),
            line_fg: color("line_fg", base.line_fg),
            line_hl: color("line_hl", base.line_hl),
            sel: color("sel", base.sel),
            current_line: color("current_line", base.current_line),
            match_bracket: color("match_bracket", base.match_bracket),
            comment: color("comment", base.comment),
            string: color("string", base.string),
            number: color("number", base.number),
            keyword: color("keyword", base.keyword),
            command: color("command", base.command),
            control: color("control", base.control),
            variable: color("variable", base.variable),
            operator: color("operator", base.operator),
            function: color("function", base.function),
            bracket: color("bracket", base.bracket),
            builtin: color("builtin", base.builtin),
            toolbar_bg: color("toolbar_bg", base.toolbar_bg),
            toolbar_fg: color("toolbar_fg", base.toolbar_fg),
            tab_bg: color("tab_bg", base.tab_bg),
            tab_active: color("tab_active", base.tab_active),
            output_bg: color("output_bg", base.output_bg),
            output_fg: color("output_fg", base.output_fg),
            output_err: color("output_err", base.output_err),
            output_ok: color("output_ok", base.output_ok),
            dock_bg: color("dock_bg", base.dock_bg),
            dock_fg: color("dock_fg", base.dock_fg),
            menu_bg: color("menu_bg", base.menu_bg),
            menu_fg: color("menu_fg", base.menu_fg),
            statusbar_bg: color("statusbar_bg", base.statusbar_bg),
            statusbar_fg: color("statusbar_fg", base.statusbar_fg),
            scrollbar: color("scrollbar", base.scrollbar),
            scrollbar_hover: color("scrollbar_hover", base.scrollbar_hover),
            autocomplete_bg: color("autocomplete_bg", base.autocomplete_bg),
            autocomplete_fg: color("autocomplete_fg", base.autocomplete_fg),
            autocomplete_sel: color("autocomplete_sel", base.autocomplete_sel),
            accent: color("accent", base.accent),
        })
    }

    pub fn token_color(&self, kind: TokenKind) -> Option<Color> {
        Some(match kind {
            TokenKind::Comment => self.comment,
            TokenKind::String => self.string,
            TokenKind::Number => self.number,
            TokenKind::Variable => self.variable,
            TokenKind::Function => self.function,
            TokenKind::Command => self.command,
            TokenKind::Control => self.control,
            TokenKind::Keyword => self.keyword,
            TokenKind::Operator => self.operator,
            TokenKind::Bracket => self.bracket,
            TokenKind::Text => return None,
        })
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::mocha()
    }
}

fn parse_hex(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

/// All-black theme — the base for parsing `mocha.toml` itself (which is
/// complete, so none of these values are ever used).
fn black() -> Theme {
    let k = Color::Rgb(0, 0, 0);
    Theme {
        name: String::new(),
        bg: k,
        fg: k,
        gutter_bg: k,
        gutter_border: k,
        line_fg: k,
        line_hl: k,
        sel: k,
        current_line: k,
        match_bracket: k,
        comment: k,
        string: k,
        number: k,
        keyword: k,
        command: k,
        control: k,
        variable: k,
        operator: k,
        function: k,
        bracket: k,
        builtin: k,
        toolbar_bg: k,
        toolbar_fg: k,
        tab_bg: k,
        tab_active: k,
        output_bg: k,
        output_fg: k,
        output_err: k,
        output_ok: k,
        dock_bg: k,
        dock_fg: k,
        menu_bg: k,
        menu_fg: k,
        statusbar_bg: k,
        statusbar_fg: k,
        scrollbar: k,
        scrollbar_hover: k,
        autocomplete_bg: k,
        autocomplete_fg: k,
        autocomplete_sel: k,
        accent: k,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_parse_fully() {
        let themes = Theme::builtins();
        assert_eq!(themes.len(), 6);
        assert_eq!(themes[0].name, "Dark (Catppuccin Mocha)");
        assert_eq!(themes[0].bg, Color::Rgb(0x1e, 0x1e, 0x2e));
        assert_eq!(themes[3].name, "Nord");
        assert_eq!(themes[3].bg, Color::Rgb(0x2e, 0x34, 0x40));
    }

    #[test]
    fn partial_theme_falls_back_to_mocha() {
        let t = Theme::parse("name = \"Half\"\nbg = \"#000000\"").unwrap();
        assert_eq!(t.name, "Half");
        assert_eq!(t.bg, Color::Rgb(0, 0, 0));
        assert_eq!(t.accent, Theme::mocha().accent); // filled from Mocha
    }

    #[test]
    fn bad_hex_is_rejected() {
        assert!(parse_hex("#12345").is_none());
        assert!(parse_hex("1e1e2e").is_none());
        assert_eq!(parse_hex("#1E1E2E"), Some(Color::Rgb(0x1e, 0x1e, 0x2e)));
    }
}
