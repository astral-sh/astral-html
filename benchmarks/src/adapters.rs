//! Equivalent owned link records, with native parsing differences left visible.

use std::borrow::Cow;
use std::convert::Infallible;
use std::sync::LazyLock;

use html5ever::tendril::TendrilSink;
use html5gum::emitters::callback::{Callback, CallbackEmitter, CallbackEvent};
use html5gum::{Emitter, ForwardingEmitter, Span, State};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Link {
    pub href: String,
    pub title: Option<String>,
    pub rel: Option<String>,
}

pub const IMPLEMENTATIONS: &[&str] = &[
    "astral-reader",
    "astral-document",
    "html5gum",
    "tl",
    "scraper",
    "lol-html",
];

static SCRAPER_LINKS: LazyLock<scraper::Selector> =
    LazyLock::new(|| scraper::Selector::parse("a[href]").unwrap());
static LOL_LINKS: LazyLock<lol_html::Selector> = LazyLock::new(|| "a[href]".parse().unwrap());

/// Compile reusable selectors before validation or timing.
pub fn prepare() {
    LazyLock::force(&SCRAPER_LINKS);
    LazyLock::force(&LOL_LINKS);
}

pub fn extract(parser: &str, input: &str) -> Result<Vec<Link>, String> {
    match parser {
        "astral-reader" => Ok(astral_reader(input)),
        "astral-document" => astral_document(input),
        "html5gum" => Ok(html5gum(input)),
        "tl" => tl(input),
        "scraper" => Ok(scraper(input)),
        "lol-html" => lol_html(input),
        _ => Err(format!("unknown parser: {parser}")),
    }
}

fn astral_reader(input: &str) -> Vec<Link> {
    astral_html::Reader::new(input)
        .filter_map(|token| {
            let astral_html::Token::StartTag(tag) = token else {
                return None;
            };
            if tag.name != "a" {
                return None;
            }
            let mut values = [None, None, None];
            for attribute in tag.attributes {
                if let Some(index) = attribute_index(attribute.name.as_bytes()) {
                    values[index] = Some(attribute.value.into_owned());
                }
            }
            link(values)
        })
        .collect()
}

fn astral_document(input: &str) -> Result<Vec<Link>, String> {
    let document = astral_html::Document::parse(input).map_err(|error| error.to_string())?;
    Ok(document
        .elements()
        .filter(|element| element.is("a"))
        .filter_map(|element| {
            let value = |name| {
                element
                    .attribute(name)
                    .map(|value| value.value().to_owned())
            };
            Some(Link {
                href: value("href")?,
                title: value("title"),
                rel: value("rel"),
            })
        })
        .collect())
}

fn link([href, title, rel]: [Option<String>; 3]) -> Option<Link> {
    Some(Link {
        href: href?,
        title,
        rel,
    })
}

fn attribute_index(name: &[u8]) -> Option<usize> {
    match name {
        b"href" => Some(0),
        b"title" => Some(1),
        b"rel" => Some(2),
        _ => None,
    }
}

struct LinkCallback<'a> {
    output: &'a mut Vec<Link>,
    attributes: Option<[Option<String>; 3]>,
    attribute: Option<usize>,
    next_state: Option<State>,
}

impl Callback<Infallible, ()> for LinkCallback<'_> {
    fn handle_event(&mut self, event: CallbackEvent<'_>, _: Span<()>) -> Option<Infallible> {
        match event {
            CallbackEvent::OpenStartTag { name } => {
                self.attributes = (name == b"a").then_some([None, None, None]);
                self.attribute = None;
                // Match Reader's HTML policy, including disabled scripting.
                self.next_state = match name {
                    b"title" | b"textarea" => Some(State::RcData),
                    b"style" | b"xmp" | b"iframe" | b"noembed" | b"noframes" => {
                        Some(State::RawText)
                    }
                    b"script" => Some(State::ScriptData),
                    b"plaintext" => Some(State::PlainText),
                    _ => None,
                };
            }
            CallbackEvent::AttributeName { name } => {
                self.attribute = attribute_index(name).filter(|&index| {
                    self.attributes
                        .as_ref()
                        .is_some_and(|values| values[index].is_none())
                });
                if let (Some(values), Some(index)) = (&mut self.attributes, self.attribute) {
                    // Mark presence now so boolean and duplicate attributes keep the first value.
                    values[index] = Some(String::new());
                }
            }
            CallbackEvent::AttributeValue { value } => {
                if let (Some(values), Some(index)) = (&mut self.attributes, self.attribute) {
                    values[index].as_mut().unwrap().push_str(
                        std::str::from_utf8(value).expect("UTF-8 input and HTML references"),
                    );
                }
            }
            CallbackEvent::CloseStartTag { .. } => {
                if let Some(record) = self.attributes.take().and_then(link) {
                    self.output.push(record);
                }
                self.attribute = None;
            }
            CallbackEvent::EndTag { .. } => self.next_state = None,
            _ => {}
        }
        None
    }
}

struct HtmlStateEmitter<'a>(CallbackEmitter<LinkCallback<'a>>);

impl ForwardingEmitter for HtmlStateEmitter<'_> {
    type Token = Infallible;

    fn inner(&mut self) -> &mut impl Emitter<Token = Infallible> {
        &mut self.0
    }

    fn emit_current_tag(&mut self) -> Option<State> {
        let _ = self.0.emit_current_tag();
        self.0.callback_mut().next_state.take()
    }

    fn should_emit_errors(&mut self) -> bool {
        false
    }

    fn emit_error(&mut self, _: html5gum::Error) {}

    // This workload only consumes link attributes. Avoid buffering unused
    // character and comment payloads, while retaining tokenizer state handling.
    fn emit_string(&mut self, _: &[u8]) {}

    fn push_comment(&mut self, _: &[u8]) {}

    fn emit_current_comment(&mut self) {}
}

fn html5gum(input: &str) -> Vec<Link> {
    let mut output = Vec::new();
    let callback = LinkCallback {
        output: &mut output,
        attributes: None,
        attribute: None,
        next_state: None,
    };
    let emitter = HtmlStateEmitter(CallbackEmitter::new(callback));
    html5gum::Tokenizer::new_with_emitter(input, emitter)
        .finish()
        .expect("reading a UTF-8 string is infallible");
    output
}

fn tl(input: &str) -> Result<Vec<Link>, String> {
    let document =
        tl::parse(input, tl::ParserOptions::default()).map_err(|error| error.to_string())?;
    Ok(document
        .nodes()
        .iter()
        .filter_map(tl::Node::as_tag)
        .filter(|element| element.name().as_bytes().eq_ignore_ascii_case(b"a"))
        .filter_map(|element| {
            let value = |name| {
                element.attributes().get(name).map(|value| {
                    value.map_or_else(String::new, |value| decode_attribute(value.as_utf8_str()))
                })
            };
            Some(Link {
                href: value("href")?,
                title: value("title"),
                rel: value("rel"),
            })
        })
        .collect())
}

/// Use the same scripting policy in DOM construction and extraction benchmarks.
pub fn scraper_document(source: &str) -> scraper::Html {
    let mut options = html5ever::ParseOpts::default();
    options.tree_builder.scripting_enabled = false;
    html5ever::parse_document(
        scraper::HtmlTreeSink::new(scraper::Html::new_document()),
        options,
    )
    .one(source)
}

fn scraper(input: &str) -> Vec<Link> {
    scraper_document(input)
        .select(&SCRAPER_LINKS)
        .map(|element| Link {
            href: element.value().attr("href").unwrap().to_owned(),
            title: element.value().attr("title").map(str::to_owned),
            rel: element.value().attr("rel").map(str::to_owned),
        })
        .collect()
}

fn lol_html(input: &str) -> Result<Vec<Link>, String> {
    let mut output = Vec::new();
    let handler = lol_html::ElementContentHandlers::default().element(
        |element: &mut lol_html::html_content::Element<'_, '_>| {
            let values = ["href", "title", "rel"].map(|name| {
                element
                    .get_attribute(name)
                    .map(|value| decode_attribute(Cow::Owned(value)))
            });
            if let Some(record) = link(values) {
                output.push(record);
            }
            Ok(())
        },
    );
    let settings = lol_html::Settings::new()
        .append_element_content_handler((Cow::Borrowed(&LOL_LINKS), handler));
    let mut rewriter = lol_html::HtmlRewriter::new(settings, |_: &[u8]| {});
    rewriter
        .write(input.as_bytes())
        .map_err(|error| error.to_string())?;
    rewriter.end().map_err(|error| error.to_string())?;
    Ok(output)
}

/// tl and lol_html expose raw attribute values. Decode those values using
/// html5ever's independent entity tables, including attribute-context lookahead.
/// This work is included in each adapter's measured extraction time.
fn decode_attribute(input: Cow<'_, str>) -> String {
    if !input.contains(['&', '\r', '\0']) {
        return input.into_owned();
    }
    let mut output = String::with_capacity(input.len());
    let mut remaining = input.as_ref();
    while let Some(index) = remaining.find(['&', '\r', '\0']) {
        output.push_str(&remaining[..index]);
        remaining = &remaining[index..];
        match remaining.as_bytes()[0] {
            b'\r' => {
                output.push('\n');
                remaining = remaining.strip_prefix("\r\n").unwrap_or(&remaining[1..]);
            }
            0 => {
                output.push('\u{fffd}');
                remaining = &remaining[1..];
            }
            b'&' => {
                remaining = &remaining[1..];
                if let Some((consumed, characters)) = attribute_reference(remaining) {
                    output.push(characters.0);
                    if let Some(second) = characters.1 {
                        output.push(second);
                    }
                    remaining = &remaining[consumed..];
                } else {
                    output.push('&');
                }
            }
            _ => unreachable!(),
        }
    }
    output.push_str(remaining);
    output
}

fn attribute_reference(input: &str) -> Option<(usize, (char, Option<char>))> {
    if let Some(numeric) = input.strip_prefix('#') {
        let (digits, radix, prefix) = match numeric.strip_prefix(['x', 'X']) {
            Some(hex) => (hex, 16, 2),
            None => (numeric, 10, 1),
        };
        let mut number = 0_u32;
        let mut count = 0;
        for character in digits.chars() {
            let Some(digit) = character.to_digit(radix) else {
                break;
            };
            number = number.saturating_mul(radix).saturating_add(digit);
            count += 1;
        }
        if count == 0 {
            return None;
        }
        let consumed = prefix + count + usize::from(digits.as_bytes().get(count) == Some(&b';'));
        let character = if (0x80..=0x9f).contains(&number) {
            html5ever::data::C1_REPLACEMENTS[(number - 0x80) as usize]
                .unwrap_or(char::from_u32(number).unwrap())
        } else {
            char::from_u32(number)
                .filter(|&value| value != '\0')
                .unwrap_or('\u{fffd}')
        };
        return Some((consumed, (character, None)));
    }

    // WHATWG names are ASCII and at most 32 bytes, including the semicolon.
    let letters = input
        .bytes()
        .take(32)
        .take_while(u8::is_ascii_alphanumeric)
        .count();
    let length =
        letters + usize::from(letters < 32 && input.as_bytes().get(letters) == Some(&b';'));
    for end in (1..=length).rev() {
        let name = &input[..end];
        if let Some(&(first, second)) = html5ever::data::NAMED_ENTITIES.get(name)
            && first != 0
        {
            if !name.ends_with(';')
                && input
                    .as_bytes()
                    .get(end)
                    .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'=')
            {
                return None;
            }
            return Some((
                end,
                (
                    char::from_u32(first).unwrap(),
                    (second != 0).then(|| char::from_u32(second).unwrap()),
                ),
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_all(input: &str, expected: Vec<Link>) {
        prepare();
        for parser in IMPLEMENTATIONS {
            assert_eq!(extract(parser, input).unwrap(), expected, "{parser}");
        }
    }

    #[test]
    fn attribute_case_and_presence_differences_remain_visible() {
        let input = "<div><A HREF='/first' href='/ignored' TITLE='hello' title='ignored' REL='tag'>one</A><span><a href='' title rel=''>two</a></span><a href>three</a><a title='ignored' rel='ignored'>skip</a></div>";
        let expected = vec![
            Link {
                href: "/first".into(),
                title: Some("hello".into()),
                rel: Some("tag".into()),
            },
            Link {
                href: "".into(),
                title: Some("".into()),
                rel: Some("".into()),
            },
            Link {
                href: "".into(),
                title: None,
                rel: None,
            },
        ];
        prepare();
        for parser in IMPLEMENTATIONS.iter().filter(|&&parser| parser != "tl") {
            assert_eq!(extract(parser, input).unwrap(), expected, "{parser}");
        }
        // Upstream tl preserves attribute case and, in this input, skips `rel`
        // after the boolean `title`. Do not repair these native differences.
        assert_eq!(
            extract("tl", input).unwrap(),
            vec![
                Link {
                    href: "/ignored".into(),
                    title: Some("ignored".into()),
                    rel: None,
                },
                Link {
                    href: "".into(),
                    title: Some("".into()),
                    rel: None,
                },
                Link {
                    href: "".into(),
                    title: None,
                    rel: None,
                },
            ],
        );
    }

    #[test]
    fn attributes_are_decoded_once_with_html_attribute_rules() {
        assert_all(
            "<a href='?a=1&amp;b=2&copy=3&notit;' title='é &copy! &#65 &#128; &NotEqualTilde;' rel='&amp;lt;'>text</a>",
            vec![Link {
                href: "?a=1&b=2&copy=3&notit;".into(),
                title: Some("é ©! A € ≂\u{338}".into()),
                rel: Some("&lt;".into()),
            }],
        );
    }

    #[test]
    fn input_newlines_and_unicode_are_preserved_correctly() {
        assert_all(
            "<a href='café/🦀' title='one\r\ntwo\rthree' rel='&#13;'>link</a>",
            vec![Link {
                href: "café/🦀".into(),
                title: Some("one\ntwo\nthree".into()),
                rel: Some("\r".into()),
            }],
        );
    }

    #[test]
    fn text_modes_keep_literal_markup_out_of_links() {
        let input = "<style><a href=style></style><textarea><a href=textarea></textarea><script>const label = '<a href=script>';</script><a href=actual>link</a>";
        let expected = vec![Link {
            href: "actual".into(),
            title: None,
            rel: None,
        }];
        for parser in IMPLEMENTATIONS.iter().filter(|&&parser| parser != "tl") {
            assert_eq!(extract(parser, input).unwrap(), expected, "{parser}");
        }
        // Native tl has no HTML text-state selection. Keep the mismatch visible.
        assert_ne!(extract("tl", input).unwrap(), expected);
    }

    #[test]
    fn disabled_scripting_is_explicit_where_supported() {
        let input = "<!doctype html><html><head></head><body><noscript><a href=fallback>link</a></noscript></body></html>";
        let expected = vec![Link {
            href: "fallback".into(),
            title: None,
            rel: None,
        }];
        for parser in IMPLEMENTATIONS
            .iter()
            .filter(|&&parser| parser != "lol-html")
        {
            assert_eq!(extract(parser, input).unwrap(), expected, "{parser}");
        }
        // lol_html has no scripting toggle; its native noscript policy differs.
        assert!(extract("lol-html", input).unwrap().is_empty());
    }

    #[test]
    fn callback_only_publishes_complete_start_tags() {
        assert_eq!(
            html5gum(
                "before &amp; <!-- <a href='/comment'> --><a href='/complete'>text</a><a href='/unfinished'"
            ),
            vec![Link {
                href: "/complete".into(),
                title: None,
                rel: None,
            }]
        );
    }

    #[test]
    fn raw_attribute_decoder_handles_references_and_unknown_names() {
        for (raw, expected) in [
            ("plain 🦀", "plain 🦀"),
            (
                "&unknown; &notin &notin; &copy=1 &copy!",
                "&unknown; &notin ∉ &copy=1 ©!",
            ),
            ("&#0; &#xD800; &#1114112; &#128;", "� � � €"),
            ("&#x; &#; &#65=", "&#x; &#; A="),
            ("a\r\nb\rc\0", "a\nb\nc�"),
            ("&amp;lt; &NotEqualTilde;", "&lt; ≂\u{338}"),
        ] {
            assert_eq!(decode_attribute(Cow::Borrowed(raw)), expected, "{raw:?}");
        }
    }
}
