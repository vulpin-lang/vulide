//! Hex-color gutter swatches: `#fff` / `#ffff` / `#ff00ff` / `#ff00ff80`
//! anywhere in a line — a string, a comment, wherever — gets a small colored
//! `●` in the gutter next to the line number, in that exact color. Plain
//! text scan, not token-aware: a color literal reads the same whether it's
//! quoted or not, so there's no real gain in restricting this to strings.

use ratatui::style::Color;

/// The first valid hex color literal on `line`, as an opaque RGB (an 8-digit
/// `#RRGGBBAA` alpha byte is dropped — there's no way to show partial
/// transparency in a single terminal cell, so the swatch is always solid).
pub fn line_color(line: &str) -> Option<Color> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end].is_ascii_hexdigit() {
                end += 1;
            }
            let len = end - start;
            if matches!(len, 3 | 4 | 6 | 8)
                && let Some(rgb) = hex_to_rgb(&chars[start..end])
            {
                return Some(Color::Rgb(rgb.0, rgb.1, rgb.2));
            }
            i = end.max(i + 1);
        } else {
            i += 1;
        }
    }
    None
}

fn hex_to_rgb(digits: &[char]) -> Option<(u8, u8, u8)> {
    let nibble = |c: char| c.to_digit(16).map(|d| d as u8);
    match digits.len() {
        // #RGB / #RGBA — each digit is a doubled nibble (0xF -> 0xFF).
        3 | 4 => {
            let r = nibble(digits[0])? * 17;
            let g = nibble(digits[1])? * 17;
            let b = nibble(digits[2])? * 17;
            Some((r, g, b))
        }
        // #RRGGBB / #RRGGBBAA
        6 | 8 => {
            let byte = |a: char, b: char| Some(nibble(a)? * 16 + nibble(b)?);
            let r = byte(digits[0], digits[1])?;
            let g = byte(digits[2], digits[3])?;
            let b = byte(digits[4], digits[5])?;
            Some((r, g, b))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_six_digit_hex_color() {
        assert_eq!(
            line_color("hex = \"#FFFFFF\""),
            Some(Color::Rgb(255, 255, 255))
        );
    }

    #[test]
    fn finds_a_three_digit_shorthand() {
        assert_eq!(line_color("border: #0f0;"), Some(Color::Rgb(0, 255, 0)));
    }

    #[test]
    fn drops_the_alpha_byte_on_eight_digits() {
        assert_eq!(line_color("#11223380"), Some(Color::Rgb(0x11, 0x22, 0x33)));
    }

    #[test]
    fn four_digit_shorthand_drops_alpha_too() {
        assert_eq!(line_color("#f00f"), Some(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn is_case_insensitive() {
        assert_eq!(line_color("#AbCdEf"), line_color("#abcdef"));
    }

    #[test]
    fn wrong_length_is_not_a_color() {
        assert_eq!(line_color("#12345"), None); // 5 digits
        assert_eq!(line_color("#1234567"), None); // 7 digits
        assert_eq!(line_color("#12"), None); // 2 digits
    }

    #[test]
    fn not_hex_digits_is_not_a_color() {
        assert_eq!(line_color("#gggggg"), None);
    }

    #[test]
    fn no_hash_is_not_a_color() {
        assert_eq!(line_color("just some text 123456"), None);
    }

    #[test]
    fn picks_the_first_match_on_a_line_with_several() {
        assert_eq!(
            line_color("#000000 then #ffffff"),
            Some(Color::Rgb(0, 0, 0))
        );
    }

    #[test]
    fn a_run_longer_than_eight_hex_digits_is_not_a_color() {
        // "#" + 9 hex digits — not any valid length, and must not be
        // mistaken for a valid 8-digit prefix.
        assert_eq!(line_color("#123456789"), None);
    }
}
