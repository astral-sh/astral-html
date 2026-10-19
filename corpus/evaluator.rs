//! Standalone evaluator compiled against either the baseline or recovery checkout.

use std::env;
use std::fs;

use astral_html::{Document, Limits};
use serde_json::{Value, json};

fn inspect(document: &Document<'_>) -> (Value, Vec<(String, Option<String>)>) {
    let mut rows = Vec::new();
    let mut roots = 0;
    let mut links = 0;
    let mut hrefs = 0;
    let mut nested_paragraphs = 0;
    let mut nested_list_items = 0;
    let mut titles = Vec::new();
    for element in document.elements() {
        let name = element.name();
        let parent = element.parent().map(|element| element.name().to_owned());
        if parent.is_none() {
            roots += 1;
        }
        if name == "a" {
            links += 1;
            if element.has_attribute("href") {
                hrefs += 1;
            }
        }
        if name == "p" && parent.as_deref() == Some("p") {
            nested_paragraphs += 1;
        }
        if name == "li" && parent.as_deref() == Some("li") {
            nested_list_items += 1;
        }
        if name == "title" && titles.len() < 3 {
            let text = element.text();
            titles.push(text.chars().take(100).collect::<String>());
        }
        rows.push((name.to_owned(), parent));
    }
    let names = rows
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    let count = |name: &str| names.iter().filter(|actual| **actual == name).count();
    let summary = json!({
        "elements": rows.len(),
        "roots": roots,
        "links": links,
        "links_with_href": hrefs,
        "nested_p": nested_paragraphs,
        "nested_li": nested_list_items,
        "html": count("html"),
        "head": count("head"),
        "body": count("body"),
        "main": count("main"),
        "table": count("table"),
        "tr": count("tr"),
        "td": count("td"),
        "title_examples": titles,
    });
    (summary, rows)
}

fn parse(source: &str, recovering: bool) -> Value {
    let default_result = if recovering {
        #[cfg(feature = "recovering")]
        {
            parse_recovering(source, None)
        }
        #[cfg(not(feature = "recovering"))]
        {
            return json!({"error": "recovery requires the recovering build feature"});
        }
    } else {
        Document::parse(source)
    };
    let default_status = default_result.as_ref().err().map(ToString::to_string);
    let result = if default_result.is_ok() {
        default_result
    } else if recovering {
        #[cfg(feature = "recovering")]
        {
            parse_recovering(
                source,
                Some(Limits {
                    max_depth: 10_000,
                    ..Limits::default()
                }),
            )
        }
        #[cfg(not(feature = "recovering"))]
        {
            return json!({"error": "recovery requires the recovering build feature"});
        }
    } else {
        Document::parse_with_limits(
            source,
            Limits {
                max_depth: 10_000,
                ..Limits::default()
            },
        )
    };
    match result {
        Ok(document) => {
            let (summary, rows) = inspect(&document);
            json!({"default_error": default_status, "summary": summary, "rows": rows})
        }
        Err(error) => json!({"default_error": default_status, "extended_error": error.to_string()}),
    }
}

#[cfg(feature = "recovering")]
fn parse_recovering<'a>(
    source: &'a str,
    limits: Option<Limits>,
) -> Result<Document<'a>, astral_html::Error> {
    match limits {
        Some(limits) => Document::parse_recovering_with_limits(source, limits),
        None => Document::parse_recovering(source),
    }
}

fn summarize(mode: &str, result: &Value) -> Value {
    let mut summary = result.clone();
    if let Some(object) = summary.as_object_mut() {
        object.remove("rows");
        object.insert("mode".to_owned(), json!(mode));
    }
    summary
}

#[cfg(feature = "recovering")]
fn differences(baseline: &Value, recovering: &Value) -> Value {
    let Some(left) = baseline["rows"].as_array() else {
        return json!({"available": false});
    };
    let Some(right) = recovering["rows"].as_array() else {
        return json!({"available": false});
    };
    let mut changed_parents = 0;
    let mut changed_names = 0;
    let mut examples = Vec::new();
    for (index, (before, after)) in left.iter().zip(right).enumerate() {
        if before[0] != after[0] {
            changed_names += 1;
        } else if before[1] != after[1] {
            changed_parents += 1;
            if examples.len() < 12 {
                examples.push(json!({
                    "index": index,
                    "element": before[0],
                    "baseline_parent": before[1],
                    "recovering_parent": after[1],
                }));
            }
        }
    }
    json!({"available": true, "changed_parents": changed_parents, "changed_names": changed_names, "examples": examples})
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = env::args().nth(1).unwrap_or_else(|| "baseline".to_owned());
    let path = env::args().nth(2).ok_or("expected path to pages")?;
    for file in env::args().skip(3) {
        let bytes = fs::read(format!("{path}/{file}"))?;
        let source = String::from_utf8_lossy(&bytes);
        let baseline = parse(&source, false);
        let name = file;
        let output = if mode == "recovering" {
            #[cfg(feature = "recovering")]
            {
                let recovering = parse(&source, true);
                json!({"file": name, "bytes": bytes.len(), "baseline": summarize("baseline", &baseline), "recovering": summarize("recovering", &recovering), "difference": differences(&baseline, &recovering)})
            }
            #[cfg(not(feature = "recovering"))]
            {
                return Err("recovering mode requires the recovering feature".into());
            }
        } else {
            json!({"file": name, "bytes": bytes.len(), "baseline": summarize("baseline", &baseline)})
        };
        println!("{output}");
    }
    Ok(())
}
