//! Document query snapshots.

use std::fmt::Write;

use astral_html::Document;

/// Show query results without depending on internal node indices or debug output.
fn describe(source: &str) -> String {
    let document = Document::parse(source).unwrap();
    describe_document(&document)
}

fn describe_document(document: &Document<'_>) -> String {
    let mut output = String::new();
    for element in document.elements() {
        writeln!(
            output,
            "<{}> parent={:?} children={:?} descendants={:?} text={:?}",
            element.name(),
            element.parent().map(|parent| parent.name()),
            element
                .children()
                .map(|child| child.name())
                .collect::<Vec<_>>(),
            element
                .descendants()
                .map(|child| child.name())
                .collect::<Vec<_>>(),
            element.text(),
        )
        .unwrap();
        for attribute in element.attributes() {
            writeln!(
                output,
                "  {} value={:?} raw={:?}",
                attribute.name,
                attribute.value(),
                attribute.raw_value(),
            )
            .unwrap();
        }
    }
    output
}

#[test]
fn recovering_optional_end_tags() {
    let source = "<main><p>first<p>second<div>block</div><ul><li>one<li><b>two</b><ul><li>inner<li>next</ul><li>last</ul><h1>title<h2>subtitle</main>";
    let document = Document::parse_recovering(source).unwrap();
    insta::assert_snapshot!(describe_document(&document), @r#"
    <main> parent=None children=["p", "p", "div", "ul", "h1", "h2"] descendants=["p", "p", "div", "ul", "li", "li", "b", "ul", "li", "li", "li", "h1", "h2"] text="firstsecondblockonetwoinnernextlasttitlesubtitle"
    <p> parent=Some("main") children=[] descendants=[] text="first"
    <p> parent=Some("main") children=[] descendants=[] text="second"
    <div> parent=Some("main") children=[] descendants=[] text="block"
    <ul> parent=Some("main") children=["li", "li", "li"] descendants=["li", "li", "b", "ul", "li", "li", "li"] text="onetwoinnernextlast"
    <li> parent=Some("ul") children=[] descendants=[] text="one"
    <li> parent=Some("ul") children=["b", "ul"] descendants=["b", "ul", "li", "li"] text="twoinnernext"
    <b> parent=Some("li") children=[] descendants=[] text="two"
    <ul> parent=Some("li") children=["li", "li"] descendants=["li", "li"] text="innernext"
    <li> parent=Some("ul") children=[] descendants=[] text="inner"
    <li> parent=Some("ul") children=[] descendants=[] text="next"
    <li> parent=Some("ul") children=[] descendants=[] text="last"
    <h1> parent=Some("main") children=[] descendants=[] text="title"
    <h2> parent=Some("main") children=[] descendants=[] text="subtitle"
    "#);
}

#[test]
fn recovering_table_and_select_scopes() {
    let source = "<table><thead><tr><th>name<th>value<tbody><tr><td>one<td><b>two<tr><td>three</table><select><optgroup label=a><option>red<option>blue<optgroup label=b><option>green</select>";
    let document = Document::parse_recovering(source).unwrap();
    insta::assert_snapshot!(describe_document(&document), @r#"
    <table> parent=None children=["thead", "tbody"] descendants=["thead", "tr", "th", "th", "tbody", "tr", "td", "td", "b", "tr", "td"] text="namevalueonetwothree"
    <thead> parent=Some("table") children=["tr"] descendants=["tr", "th", "th"] text="namevalue"
    <tr> parent=Some("thead") children=["th", "th"] descendants=["th", "th"] text="namevalue"
    <th> parent=Some("tr") children=[] descendants=[] text="name"
    <th> parent=Some("tr") children=[] descendants=[] text="value"
    <tbody> parent=Some("table") children=["tr", "tr"] descendants=["tr", "td", "td", "b", "tr", "td"] text="onetwothree"
    <tr> parent=Some("tbody") children=["td", "td"] descendants=["td", "td", "b"] text="onetwo"
    <td> parent=Some("tr") children=[] descendants=[] text="one"
    <td> parent=Some("tr") children=["b"] descendants=["b"] text="two"
    <b> parent=Some("td") children=[] descendants=[] text="two"
    <tr> parent=Some("tbody") children=["td"] descendants=["td"] text="three"
    <td> parent=Some("tr") children=[] descendants=[] text="three"
    <select> parent=None children=["optgroup", "optgroup"] descendants=["optgroup", "option", "option", "optgroup", "option"] text="redbluegreen"
    <optgroup> parent=Some("select") children=["option", "option"] descendants=["option", "option"] text="redblue"
      label value="a" raw=Some("a")
    <option> parent=Some("optgroup") children=[] descendants=[] text="red"
    <option> parent=Some("optgroup") children=[] descendants=[] text="blue"
    <optgroup> parent=Some("select") children=["option"] descendants=["option"] text="green"
      label value="b" raw=Some("b")
    <option> parent=Some("optgroup") children=[] descendants=[] text="green"
    "#);
}

#[test]
fn lexical_scopes_and_descendant_text() {
    // Paragraphs retain their source scopes; </section> closes both of them.
    let source = "<main>before<section><p>one<p>two</section>after</main><aside>end</aside>";
    insta::assert_snapshot!(describe(source), @r#"
    <main> parent=None children=["section"] descendants=["section", "p", "p"] text="beforeonetwoafter"
    <section> parent=Some("main") children=["p"] descendants=["p", "p"] text="onetwo"
    <p> parent=Some("section") children=["p"] descendants=["p"] text="onetwo"
    <p> parent=Some("p") children=[] descendants=[] text="two"
    <aside> parent=None children=[] descendants=[] text="end"
    "#);
}

#[test]
fn void_elements_and_nonvoid_self_closing_flags() {
    let source = "<div/>one<br>two<img src='icon.svg'/><span/>three</div><hr>";
    insta::assert_snapshot!(describe(source), @r#"
    <div> parent=None children=["br", "img", "span"] descendants=["br", "img", "span"] text="onetwothree"
    <br> parent=Some("div") children=[] descendants=[] text=""
    <img> parent=Some("div") children=[] descendants=[] text=""
      src value="icon.svg" raw=Some("icon.svg")
    <span> parent=Some("div") children=[] descendants=[] text="three"
    <hr> parent=None children=[] descendants=[] text=""
    "#);
}

#[test]
fn attribute_case_duplicates_and_source_spelling() {
    let source = "<A HrEf='?x=1&amp;y=2' HREF=ignored FLAG empty='' unquoted=value>Link</A>";
    insta::assert_snapshot!(describe(source), @r#"
    <a> parent=None children=[] descendants=[] text="Link"
      href value="?x=1&y=2" raw=Some("?x=1&amp;y=2")
      flag value="" raw=None
      empty value="" raw=Some("")
      unquoted value="value" raw=Some("value")
    "#);
}

#[test]
fn attribute_queries_distinguish_boolean_empty_and_absent() {
    let document = Document::parse("<a href></a><a HREF=''></a><a></a>").unwrap();
    let mut output = String::new();
    for element in document.elements() {
        writeln!(
            output,
            "is(A)={} has(HREF)={} attribute(Href)={:?}",
            element.is("A"),
            element.has_attribute("HREF"),
            element
                .attribute("Href")
                .map(|attribute| (attribute.value(), attribute.raw_value())),
        )
        .unwrap();
    }
    insta::assert_snapshot!(output, @r#"
    is(A)=true has(HREF)=true attribute(Href)=Some(("", None))
    is(A)=true has(HREF)=true attribute(Href)=Some(("", Some("")))
    is(A)=true has(HREF)=false attribute(Href)=None
    "#);
}

#[test]
fn character_references_decode_once_in_context() {
    let source = "<p title='&amp;lt; &#x41; &notit; &amp=1'>&amp;lt; &#x41; &notit; &amp=1</p>";
    insta::assert_snapshot!(describe(source), @r#"
    <p> parent=None children=[] descendants=[] text="&lt; A ¬it; &=1"
      title value="&lt; A &notit; &amp=1" raw=Some("&amp;lt; &#x41; &notit; &amp=1")
    "#);
}

#[test]
fn newlines_and_unicode_preserve_source_and_decoded_values() {
    let source = "<DÍV TITLE='café\r\n🦀\ré'>café\r\n<b>東京</b>\ré</DÍV>";
    insta::assert_snapshot!(describe(source), @r#"
    <dÍv> parent=None children=["b"] descendants=["b"] text="café\n東京\né"
      title value="café\n🦀\né" raw=Some("café\r\n🦀\ré")
    <b> parent=Some("dÍv") children=[] descendants=[] text="東京"
    "#);
}

#[test]
fn comments_doctypes_and_processing_instructions_are_omitted() {
    let source =
        "<!doctype html><?render ready?><p>one<!--hidden--><?render ignored?><b>two</b>three</p>";
    insta::assert_snapshot!(describe(source), @r#"
    <p> parent=None children=["b"] descendants=["b"] text="onetwothree"
    <b> parent=Some("p") children=[] descendants=[] text="two"
    "#);
}

#[test]
fn unmatched_end_tags_and_incomplete_start_tags() {
    let source = "<div><span>one</div></missing><p>two<a href='unfinished";
    insta::assert_snapshot!(describe(source), @r#"
    <div> parent=None children=["span"] descendants=["span"] text="one"
    <span> parent=Some("div") children=[] descendants=[] text="one"
    <p> parent=None children=[] descendants=[] text="two"
    "#);
}

#[test]
fn raw_text_and_rcdata_keep_markup_as_text() {
    let source = "<script>&amp;<b>code</b></script><style>&amp;<i>css</i></style><title>&amp;<b>title</b></title><textarea>&lt;b&gt;text&lt;/b&gt;</textarea>";
    insta::assert_snapshot!(describe(source), @r#"
    <script> parent=None children=[] descendants=[] text="&amp;<b>code</b>"
    <style> parent=None children=[] descendants=[] text="&amp;<i>css</i>"
    <title> parent=None children=[] descendants=[] text="&<b>title</b>"
    <textarea> parent=None children=[] descendants=[] text="<b>text</b>"
    "#);
}

#[test]
fn noscript_contains_markup_and_plaintext_runs_to_eof() {
    let source = "<noscript><b>visible</b></noscript><plaintext>&amp;<b>literal</b>";
    insta::assert_snapshot!(describe(source), @r#"
    <noscript> parent=None children=["b"] descendants=["b"] text="visible"
    <b> parent=Some("noscript") children=[] descendants=[] text="visible"
    <plaintext> parent=None children=[] descendants=[] text="&amp;<b>literal</b>"
    "#);
}
