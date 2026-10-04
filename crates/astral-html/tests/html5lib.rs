//! Token-output conformance against the pinned html5lib tokenizer suite.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use astral_html::{State, Token, Tokenizer};
use serde_json::{Value, json};

/// Decode the additional escapes used by html5lib's non-scalar test cases.
fn unescape(input: &str) -> Option<String> {
    let mut output = String::new();
    let mut remaining = input;
    while let Some(index) = remaining.find("\\u") {
        output.push_str(&remaining[..index]);
        let escape = &remaining[index + 2..];
        let code = u32::from_str_radix(escape.get(..4)?, 16).ok()?;
        output.push(char::from_u32(code)?);
        remaining = &escape[4..];
    }
    output.push_str(remaining);
    Some(output)
}

/// Apply html5lib's double escaping to every expected string, including attribute keys.
fn unescape_value(value: &mut Value) -> Option<()> {
    match value {
        Value::String(value) => *value = unescape(value)?,
        Value::Array(values) => {
            for value in values {
                unescape_value(value)?;
            }
        }
        Value::Object(values) => {
            let original = std::mem::take(values);
            for (name, mut value) in original {
                unescape_value(&mut value)?;
                values.insert(unescape(&name)?, value);
            }
        }
        _ => {}
    }
    Some(())
}

/// Convert tokens to the fixture representation, coalescing adjacent text.
fn output(source: &str, state: State, last_start_tag: Option<&str>) -> Value {
    let mut result = Vec::<Value>::new();
    for token in Tokenizer::with_state(source, state, last_start_tag) {
        let value = match token {
            Token::Text(text) => {
                if text.is_empty() {
                    continue;
                }
                if let Some(last) = result.last_mut().filter(|last| last[0] == "Character") {
                    let mut merged = last[1].as_str().unwrap().to_owned();
                    merged.push_str(&text);
                    last[1] = Value::String(merged);
                    continue;
                }
                json!(["Character", text])
            }
            Token::StartTag(tag) => {
                let attributes: BTreeMap<_, _> = tag
                    .attributes
                    .into_iter()
                    .map(|attribute| (attribute.name.into_owned(), attribute.value.into_owned()))
                    .collect();
                if tag.self_closing {
                    json!(["StartTag", tag.name, attributes, true])
                } else {
                    json!(["StartTag", tag.name, attributes])
                }
            }
            Token::EndTag(tag) => json!(["EndTag", tag.name]),
            Token::Comment(comment) => json!(["Comment", comment]),
            Token::Doctype(doctype) => json!([
                "DOCTYPE",
                doctype.name,
                doctype.public_id,
                doctype.system_id,
                !doctype.force_quirks
            ]),
            Token::ProcessingInstruction { target, data } => {
                json!(["ProcessingInstruction", target, data])
            }
        };
        result.push(value);
    }
    Value::Array(result)
}

#[test]
fn html5lib_tokenizer() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/html5lib/tokenizer");
    let mut paths: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "test")
        })
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 14, "the pinned suite must be present in full");
    let mut runs = 0;
    let mut excluded_surrogates = 0;
    let mut excluded_xml = 0;
    let mut failures = Vec::new();
    for path in paths {
        let fixture: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if path.file_name().unwrap() == "xmlViolation.test" {
            excluded_xml += fixture["xmlViolationTests"].as_array().unwrap().len();
            continue;
        }
        for (index, test) in fixture["tests"].as_array().unwrap().iter().enumerate() {
            let double_escaped = test["doubleEscaped"].as_bool().unwrap_or(false);
            let mut source = test["input"].as_str().unwrap().to_owned();
            let mut expected = test["output"].clone();
            if double_escaped {
                let Some(decoded) = unescape(&source) else {
                    excluded_surrogates += 1;
                    continue;
                };
                source = decoded;
                assert!(
                    unescape_value(&mut expected).is_some(),
                    "scalar input must have scalar output"
                );
            }
            let states = test["initialStates"]
                .as_array()
                .cloned()
                .unwrap_or_else(|| vec![json!("Data state")]);
            for state in states {
                let state = match state.as_str().unwrap() {
                    "Data state" => State::Data,
                    "RCDATA state" => State::Rcdata,
                    "RAWTEXT state" => State::Rawtext,
                    "Script data state" => State::ScriptData,
                    "PLAINTEXT state" => State::Plaintext,
                    "CDATA section state" => State::Cdata,
                    state => panic!("unknown state: {state}"),
                };
                runs += 1;
                let actual = output(&source, state, test["lastStartTag"].as_str());
                if actual != expected {
                    failures.push(format!("{}:{index} {state:?}: {}\ninput: {source:?}\nexpected: {expected}\nactual:   {actual}", path.file_name().unwrap().to_string_lossy(), test["description"]));
                }
            }
        }
    }
    eprintln!(
        "html5lib: {runs} runs, {excluded_surrogates} non-scalar inputs excluded, {excluded_xml} XML-coercion cases excluded"
    );
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    assert_eq!(runs, 7045, "unexpected tokenizer coverage");
    assert_eq!(excluded_surrogates, 4);
    assert_eq!(excluded_xml, 4);
}
