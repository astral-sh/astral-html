//! HTML character references and input newline normalization.

use std::borrow::Cow;

use crate::entities_data::NAMED;

/// Decode HTML character references and normalize input CRLF and CR to LF.
///
/// In attribute mode, NUL becomes U+FFFD and semicolonless named references are left
/// intact before ASCII letters, digits, or `=`. Unchanged input is borrowed.
/// References are decoded once: `&amp;lt;` becomes `&lt;`.
#[inline]
pub fn decode(input: &str, attribute: bool) -> Cow<'_, str> {
    let bytes = input.as_bytes();
    let first = if bytes.len() < 16 {
        bytes
            .iter()
            .position(|&byte| matches!(byte, b'&' | b'\r') || (attribute && byte == 0))
    } else if attribute {
        memchr::memchr3(b'&', b'\r', 0, bytes)
    } else {
        memchr::memchr2(b'&', b'\r', bytes)
    };
    let Some(first) = first else {
        return Cow::Borrowed(input);
    };
    decode_from(input, attribute, first)
}

/// Start at an ASCII marker from the initial scan; preceding bytes need no transformation.
#[inline(never)]
fn decode_from(input: &str, attribute: bool, mut cursor: usize) -> Cow<'_, str> {
    let bytes = input.as_bytes();
    let mut output = None;
    let mut unchanged = 0;
    while cursor < bytes.len() {
        let (next, replacement) = match bytes[cursor] {
            b'&' => {
                let Some((consumed, replacement)) = reference(&input[cursor + 1..], attribute)
                else {
                    cursor += 1;
                    continue;
                };
                (cursor + consumed + 1, replacement)
            }
            b'\r' => (
                cursor + 1 + usize::from(bytes.get(cursor + 1) == Some(&b'\n')),
                Replacement::Character('\n'),
            ),
            0 if attribute => (cursor + 1, Replacement::Character('\u{fffd}')),
            _ => {
                cursor += if attribute {
                    memchr::memchr3(b'&', b'\r', 0, &bytes[cursor..])
                } else {
                    memchr::memchr2(b'&', b'\r', &bytes[cursor..])
                }
                .unwrap_or(bytes.len() - cursor);
                continue;
            }
        };
        let output = output.get_or_insert_with(|| String::with_capacity(input.len()));
        output.push_str(&input[unchanged..cursor]);
        match replacement {
            Replacement::Character(character) => output.push(character),
            Replacement::Named(text) => output.push_str(text),
        }
        cursor = next;
        unchanged = next;
    }
    if let Some(mut output) = output {
        output.push_str(&input[unchanged..]);
        // Numeric references can consume arbitrarily many digits for one character.
        // Keep small buffers to avoid reallocating ordinary short attribute values.
        if output.capacity() > 64 && output.capacity() > output.len().saturating_mul(2) {
            output.shrink_to_fit();
        }
        Cow::Owned(output)
    } else {
        Cow::Borrowed(input)
    }
}

/// Normalize HTML input newlines, leaving character references and NUL intact.
pub fn normalize(input: &str) -> Cow<'_, str> {
    let Some(first) = memchr::memchr(b'\r', input.as_bytes()) else {
        return Cow::Borrowed(input);
    };
    let mut output = String::with_capacity(input.len());
    output.push_str(&input[..first]);
    let mut rest = &input[first..];
    while let Some(offset) = memchr::memchr(b'\r', rest.as_bytes()) {
        output.push_str(&rest[..offset]);
        output.push('\n');
        rest = &rest[offset + 1..];
        if let Some(suffix) = rest.strip_prefix('\n') {
            rest = suffix;
        }
    }
    output.push_str(rest);
    Cow::Owned(output)
}

enum Replacement {
    Character(char),
    Named(&'static str),
}

/// Decode a reference after `&`, returning the number of bytes consumed after it.
fn reference(input: &str, attribute: bool) -> Option<(usize, Replacement)> {
    let bytes = input.as_bytes();
    if bytes.first() == Some(&b'#') {
        let hex = matches!(bytes.get(1), Some(b'x' | b'X'));
        let start = if hex { 2 } else { 1 };
        let radix = if hex { 16 } else { 10 };
        let mut end = start;
        let mut number = 0u32;
        while let Some(digit) = bytes
            .get(end)
            .and_then(|byte| char::from(*byte).to_digit(radix))
        {
            number = number.saturating_mul(radix).saturating_add(digit);
            end += 1;
        }
        if end == start {
            return None;
        }
        if bytes.get(end) == Some(&b';') {
            end += 1;
        }
        return Some((end, Replacement::Character(numeric(number))));
    }

    // Common semicolon-terminated references need no contextual lookahead.
    let common = match bytes {
        [b'g', b't', b';', ..] => Some((3, ">")),
        [b'l', b't', b';', ..] => Some((3, "<")),
        [b'a', b'm', b'p', b';', ..] => Some((4, "&")),
        [b'q', b'u', b'o', b't', b';', ..] => Some((5, "\"")),
        [b'a', b'p', b'o', b's', b';', ..] => Some((5, "'")),
        _ => None,
    };
    if let Some((length, value)) = common {
        return Some((length, Replacement::Named(value)));
    }

    // Bound unknown names by the longest WHATWG reference (32 bytes including `;`).
    let mut end = 0;
    while end < 32 && bytes.get(end).is_some_and(u8::is_ascii_alphanumeric) {
        end += 1;
    }
    if end < 32 && bytes.get(end) == Some(&b';') {
        end += 1;
    }
    for length in (1..=end).rev() {
        let candidate = &input[..length];
        if let Ok(index) = NAMED.binary_search_by_key(&candidate, |(name, _)| *name) {
            if !candidate.ends_with(';')
                && attribute
                && bytes
                    .get(length)
                    .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'=')
            {
                return None;
            }
            return Some((length, Replacement::Named(NAMED[index].1)));
        }
    }
    None
}

/// Apply the numeric reference replacement rules, including Windows-1252.
fn numeric(number: u32) -> char {
    const C1: [u32; 32] = [
        0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039,
        0x152, 0x8d, 0x17d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
        0x2dc, 0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
    ];
    let number = match number {
        0 => 0xfffd,
        0x80..=0x9f => C1[(number - 0x80) as usize],
        number => number,
    };
    char::from_u32(number).unwrap_or('\u{fffd}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_references_follow_attribute_context() {
        assert_eq!(
            decode("&notit; &amp=1 &AMP; &NotEqualTilde;", false),
            "¬it; &=1 & ≂\u{338}"
        );
        assert_eq!(
            decode("&notit; &amp=1 &AMP; &NotEqualTilde;", true),
            "&notit; &amp=1 & ≂\u{338}"
        );
        assert_eq!(decode("&amp;lt;", true), "&lt;");
    }

    #[test]
    fn unchanged_ampersands_borrow_the_input() {
        for source in [
            "a & b",
            "?a=1&unknown=2",
            "&unknown;",
            "&#x;",
            "é &unknown; 🦀",
        ] {
            for attribute in [false, true] {
                assert!(
                    matches!(decode(source, attribute), Cow::Borrowed(value) if value == source)
                );
            }
        }
        assert!(matches!(decode("&notit;", true), Cow::Borrowed("&notit;")));
        assert_eq!(
            decode("&unknown; then &amp; and &unknown;", false),
            "&unknown; then & and &unknown;"
        );
    }

    #[test]
    fn numeric_references_replace_invalid_scalars() {
        assert_eq!(
            decode("&#0; &#xD800; &#1114112; &#128; &#x1F980;", false),
            "� � � € 🦀"
        );
        assert_eq!(decode("&#999999999999999999999999;", true), "�");
        assert_eq!(decode("&#x; &#; &#65=", true), "&#x; &#; A=");
    }

    #[test]
    fn compressed_references_do_not_retain_input_sized_buffers() {
        for (source, expected) in [
            (format!("&#{}65;", "0".repeat(1024 * 1024)), "A".to_owned()),
            (format!("&#x{}41;", "0".repeat(1024 * 1024)), "A".to_owned()),
            ("&#65;".repeat(8192), "A".repeat(8192)),
            (
                "&CounterClockwiseContourIntegral;".repeat(8192),
                "∳".repeat(8192),
            ),
        ] {
            for attribute in [false, true] {
                let Cow::Owned(decoded) = decode(&source, attribute) else {
                    panic!("decoded references must be owned");
                };
                assert_eq!(decoded, expected);
                assert!(decoded.capacity() <= 2 * decoded.len());
            }
        }
    }

    #[test]
    fn newlines_and_null_are_contextual() {
        assert_eq!(decode("a\r\nb\rc\0", true), "a\nb\nc�");
        assert_eq!(decode("a\r\nb\rc\0", false), "a\nb\nc\0");
        assert_eq!(normalize("\r\n\r&copy;\0"), "\n\n&copy;\0");
        assert!(matches!(decode("plain 🦀 text", true), Cow::Borrowed(_)));
    }

    #[test]
    fn every_named_reference_decodes() {
        assert_eq!(NAMED.len(), 2231);
        for &(name, expected) in NAMED {
            assert_eq!(decode(&format!("&{name}"), false), expected, "{name}");
        }
    }
}
