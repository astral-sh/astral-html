//! Small, readable examples of Reader and fragment-tokenizer contracts.

use std::borrow::Cow;
use std::fmt::Write;

use astral_html::{Reader, State, Token, Tokenizer};

fn snapshot<'a>(mut next: impl FnMut() -> (Option<Token<'a>>, usize)) -> String {
    let mut output = String::new();
    loop {
        let (token, position) = next();
        if let Some(token) = token {
            writeln!(output, "{position}: {token:?}").unwrap();
        } else {
            writeln!(output, "{position}: EOF").unwrap();
            return output;
        }
    }
}

fn read(source: &str) -> String {
    let mut reader = Reader::new(source);
    snapshot(|| (reader.next(), reader.position()))
}

fn fragment(source: &str, state: State, last_start_tag: Option<&str>) -> String {
    let mut tokenizer = Tokenizer::with_state(source, state, last_start_tag);
    snapshot(|| (tokenizer.next(), tokenizer.position()))
}

#[test]
fn reader_mixed_content() {
    let source = "<!doctype html><?note ok?>\n<!--intro--><DIV id=main disabled empty=\"\" title='A &amp; B'>Hello <b>世界</b>!</DIV>";
    insta::assert_snapshot!(read(source), @r#"
    15: Doctype(Doctype { name: Some("html"), public_id: None, system_id: None, force_quirks: false })
    26: ProcessingInstruction { target: "note", data: "ok" }
    27: Text("\n")
    39: Comment("intro")
    88: StartTag(Tag { name: "div", attributes: [Attribute { name: "id", value: "main", raw_value: Some("main") }, Attribute { name: "disabled", value: "", raw_value: None }, Attribute { name: "empty", value: "", raw_value: Some("") }, Attribute { name: "title", value: "A & B", raw_value: Some("A &amp; B") }], self_closing: false })
    94: Text("Hello ")
    97: StartTag(Tag { name: "b", attributes: [], self_closing: false })
    103: Text("世界")
    107: EndTag(Tag { name: "b", attributes: [], self_closing: false })
    108: Text("!")
    114: EndTag(Tag { name: "div", attributes: [], self_closing: false })
    114: EOF
    "#);
}

#[test]
fn reader_rcdata() {
    let source =
        "<title>A &amp; <b>literal</b></TITLE><textarea>\r\n&lt;note&gt;</textarea><p>after</p>";
    insta::assert_snapshot!(read(source), @r#"
    7: StartTag(Tag { name: "title", attributes: [], self_closing: false })
    29: Text("A & <b>literal</b>")
    37: EndTag(Tag { name: "title", attributes: [], self_closing: false })
    47: StartTag(Tag { name: "textarea", attributes: [], self_closing: false })
    61: Text("\n<note>")
    72: EndTag(Tag { name: "textarea", attributes: [], self_closing: false })
    75: StartTag(Tag { name: "p", attributes: [], self_closing: false })
    80: Text("after")
    84: EndTag(Tag { name: "p", attributes: [], self_closing: false })
    84: EOF
    "#);
}

#[test]
fn reader_raw_text() {
    let source = "<style>a &amp; <b></style><xmp><i>literal</i></xmp><p>after</p>";
    insta::assert_snapshot!(read(source), @r#"
    7: StartTag(Tag { name: "style", attributes: [], self_closing: false })
    18: Text("a &amp; <b>")
    26: EndTag(Tag { name: "style", attributes: [], self_closing: false })
    31: StartTag(Tag { name: "xmp", attributes: [], self_closing: false })
    45: Text("<i>literal</i>")
    51: EndTag(Tag { name: "xmp", attributes: [], self_closing: false })
    54: StartTag(Tag { name: "p", attributes: [], self_closing: false })
    59: Text("after")
    63: EndTag(Tag { name: "p", attributes: [], self_closing: false })
    63: EOF
    "#);
}

#[test]
fn reader_script_escapes() {
    let source = "<script><!--<script>literal</script>--></script><p>after</p>";
    insta::assert_snapshot!(read(source), @r#"
    8: StartTag(Tag { name: "script", attributes: [], self_closing: false })
    39: Text("<!--<script>literal</script>-->")
    48: EndTag(Tag { name: "script", attributes: [], self_closing: false })
    51: StartTag(Tag { name: "p", attributes: [], self_closing: false })
    56: Text("after")
    60: EndTag(Tag { name: "p", attributes: [], self_closing: false })
    60: EOF
    "#);
}

#[test]
fn reader_noscript_and_plaintext() {
    let source = "<noscript><b>fallback</b></noscript><plaintext>A &amp; <i>literal</i>";
    insta::assert_snapshot!(read(source), @r#"
    10: StartTag(Tag { name: "noscript", attributes: [], self_closing: false })
    13: StartTag(Tag { name: "b", attributes: [], self_closing: false })
    21: Text("fallback")
    25: EndTag(Tag { name: "b", attributes: [], self_closing: false })
    36: EndTag(Tag { name: "noscript", attributes: [], self_closing: false })
    47: StartTag(Tag { name: "plaintext", attributes: [], self_closing: false })
    69: Text("A &amp; <i>literal</i>")
    69: EOF
    "#);
}

#[test]
fn tokenizer_fragment_states() {
    insta::assert_snapshot!(fragment("é\r\n&amp;<b>literal</b></TiTlE><p>after</p>", State::Rcdata, Some("TITLE")), @r#"
    23: Text("é\n&<b>literal</b>")
    31: EndTag(Tag { name: "title", attributes: [], self_closing: false })
    34: StartTag(Tag { name: "p", attributes: [], self_closing: false })
    39: Text("after")
    43: EndTag(Tag { name: "p", attributes: [], self_closing: false })
    43: EOF
    "#);
    insta::assert_snapshot!(fragment("é\r\n&amp;</style><p>after</p>", State::Rawtext, Some("style")), @r#"
    9: Text("é\n&amp;")
    17: EndTag(Tag { name: "style", attributes: [], self_closing: false })
    20: StartTag(Tag { name: "p", attributes: [], self_closing: false })
    25: Text("after")
    29: EndTag(Tag { name: "p", attributes: [], self_closing: false })
    29: EOF
    "#);
    insta::assert_snapshot!(fragment("<!-- text --></script><p>after</p>", State::ScriptData, Some("script")), @r#"
    13: Text("<!-- text -->")
    22: EndTag(Tag { name: "script", attributes: [], self_closing: false })
    25: StartTag(Tag { name: "p", attributes: [], self_closing: false })
    30: Text("after")
    34: EndTag(Tag { name: "p", attributes: [], self_closing: false })
    34: EOF
    "#);
    insta::assert_snapshot!(fragment("é\r\n&amp;<p>literal</p>", State::Plaintext, None), @r#"
    23: Text("é\n&amp;<p>literal</p>")
    23: EOF
    "#);
    insta::assert_snapshot!(fragment("é\r\n&amp;<b>literal</b>]]><p>after</p>", State::Cdata, None), @r#"
    26: Text("é\n&amp;<b>literal</b>")
    29: StartTag(Tag { name: "p", attributes: [], self_closing: false })
    34: Text("after")
    38: EndTag(Tag { name: "p", attributes: [], self_closing: false })
    38: EOF
    "#);
}

#[test]
fn cloned_tokenizers_keep_independent_positions_and_context() {
    let mut tokenizer = Tokenizer::new("<title>A &amp; <b>literal</b></title>");
    assert!(matches!(tokenizer.next(), Some(Token::StartTag(tag)) if tag.name == "title"));
    {
        let context = String::from("TITLE");
        tokenizer.set_state(State::Rcdata, Some(&context));
    }
    let mut text_mode = tokenizer.clone();
    tokenizer.set_state(State::Data, None);
    assert_eq!(tokenizer.position(), 7);
    assert_eq!(text_mode.position(), 7);
    assert_eq!(tokenizer.next(), Some(Token::Text(Cow::Borrowed("A & "))));
    assert!(matches!(tokenizer.next(), Some(Token::StartTag(tag)) if tag.name == "b"));
    assert_eq!(text_mode.position(), 7);
    assert_eq!(
        text_mode.next(),
        Some(Token::Text(Cow::Borrowed("A & <b>literal</b>")))
    );
    assert!(matches!(text_mode.next(), Some(Token::EndTag(tag)) if tag.name == "title"));
    assert_eq!(text_mode.next(), None);
}
