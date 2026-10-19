//! Full-document parser comparison over the same preloaded UTF-8 inputs.

use std::env;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use html5ever::{ParseOpts, tendril::TendrilSink};
use markup5ever_rcdom::RcDom;

#[derive(Clone, Copy)]
enum Parser {
    #[cfg(not(feature = "resilient"))]
    Baseline,
    #[cfg(not(feature = "resilient"))]
    BaselineExpanded,
    #[cfg(feature = "resilient")]
    Resilient,
    Servo,
}

impl Parser {
    fn name(self) -> &'static str {
        match self {
            #[cfg(not(feature = "resilient"))]
            Self::Baseline => "astral-baseline",
            #[cfg(not(feature = "resilient"))]
            Self::BaselineExpanded => "astral-baseline-expanded-depth",
            #[cfg(feature = "resilient")]
            Self::Resilient => "astral-recovering",
            Self::Servo => "servo-html5ever-rcdom",
        }
    }

    fn parse(self, input: &str) -> Result<(), String> {
        match self {
            #[cfg(not(feature = "resilient"))]
            Self::Baseline => {
                let document = astral_html::Document::parse(black_box(input))
                    .map_err(|error| error.to_string())?;
                black_box(document);
            }
            #[cfg(not(feature = "resilient"))]
            Self::BaselineExpanded => {
                let limits = astral_html::Limits {
                    max_depth: 10_000,
                    ..Default::default()
                };
                let document = astral_html::Document::parse_with_limits(black_box(input), limits)
                    .map_err(|error| error.to_string())?;
                black_box(document);
            }
            #[cfg(feature = "resilient")]
            Self::Resilient => {
                let document = astral_html::Document::parse_recovering(black_box(input))
                    .map_err(|error| error.to_string())?;
                black_box(document);
            }
            Self::Servo => {
                let document: RcDom =
                    html5ever::parse_document(RcDom::default(), ParseOpts::default())
                        .one(black_box(input));
                black_box(document);
            }
        }
        Ok(())
    }
}

struct Case {
    name: String,
    input: String,
}

fn read_cases(directory: &Path, cases: &mut Vec<Case>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            read_cases(&path, cases)?;
        } else if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("html" | "htm")
        ) {
            let bytes = fs::read(&path)?;
            let input = String::from_utf8_lossy(&bytes).into_owned();
            cases.push(Case {
                name: path.display().to_string(),
                input,
            });
        }
    }
    Ok(())
}

fn benchmark(parser: Parser, input: &str) -> Result<(u64, u32), String> {
    // Check success first: a fast resource-limit error is not a completed parse.
    parser.parse(input)?;

    // Calibrate outside samples so each sample includes sufficient work to
    // overwhelm timer overhead without running arbitrarily long on tiny files.
    let start = Instant::now();
    let mut iterations = 0_u32;
    while start.elapsed() < Duration::from_millis(30) && iterations < 100_000 {
        parser.parse(input)?;
        iterations += 1;
    }

    let mut samples = [0_u64; 5];
    for sample in &mut samples {
        let start = Instant::now();
        for _ in 0..iterations {
            parser.parse(input)?;
        }
        *sample = (start.elapsed().as_nanos() / u128::from(iterations)) as u64;
    }
    samples.sort_unstable();
    Ok((samples[2], iterations))
}

fn synthetic_cases() -> Vec<Case> {
    [
        (
            "well-formed-index",
            format!("<html><body>{}</body></html>", "<a href=/demo.whl>demo</a><br>".repeat(1_000)),
        ),
        (
            "optional-li-and-p",
            format!("<html><body>{}</body></html>", "<ul><li>one<li>two</ul><p>alpha<p>beta".repeat(1_000)),
        ),
        (
            "misnested-formatting",
            format!("<html><body>{}</body></html>", "<p><b>A<i>B</b>C</i></p>".repeat(1_000)),
        ),
        (
            "table-foster-parenting",
            format!("<html><body>{}</body></html>", "<table>stray<tr><td>a<td>b</table>".repeat(1_000)),
        ),
        (
            "unmatched-end-tags",
            format!(
                "{}{}text{}",
                "<section>".repeat(120),
                "</absent>".repeat(1_000),
                "</section>".repeat(120),
            ),
        ),
        (
            "entity-attributes",
            format!("<html><body>{}</body></html>", "<a href='/demo.whl?x=&amp;&quot;&gt;&#65;&#x1F980;' data-extra='&CounterClockwiseContourIntegral;&NotEqual;'>demo</a>".repeat(1_000)),
        ),
        (
            "script-text",
            format!("<html><body><script>{}</script></body></html>", "const sample = '<a>';\n".repeat(8_192)),
        ),
        (
            "over-depth-limit",
            format!("{}text{}", "<section>".repeat(300), "</section>".repeat(300)),
        ),
    ]
    .into_iter()
    .map(|(name, input)| Case {
        name: format!("synthetic/{name}.html"),
        input,
    })
    .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: astral-servo-comparison DIRECTORY_OF_HTML_FILES")?;
    let mut cases = Vec::new();
    read_cases(&directory, &mut cases)?;
    let public_case_count = cases.len();
    cases.extend(synthetic_cases());
    cases.sort_by(|left, right| left.name.cmp(&right.name));
    if public_case_count == 0 {
        return Err(format!("no HTML files under {}", directory.display()).into());
    }

    #[cfg(not(feature = "resilient"))]
    let parsers = [Parser::Baseline, Parser::Servo];
    #[cfg(feature = "resilient")]
    let parsers = [Parser::Resilient, Parser::Servo];

    println!("case\tbytes\tparser\tstatus\tmedian_ns\titerations");
    for case in cases {
        for parser in parsers {
            match benchmark(parser, &case.input) {
                Ok((median_ns, iterations)) => println!(
                    "{}\t{}\t{}\tok\t{median_ns}\t{iterations}",
                    case.name,
                    case.input.len(),
                    parser.name(),
                ),
                Err(error) => println!(
                    "{}\t{}\t{}\t{}\t\t",
                    case.name,
                    case.input.len(),
                    parser.name(),
                    error.replace('\t', " ").replace('\n', " "),
                ),
            }
        }
        // This extra measurement makes deep malformed pages comparable with
        // recovery without disguising a default-limit failure as success.
        #[cfg(not(feature = "resilient"))]
        if Parser::Baseline.parse(&case.input).is_err() {
            let parser = Parser::BaselineExpanded;
            match benchmark(parser, &case.input) {
                Ok((median_ns, iterations)) => println!(
                    "{}\t{}\t{}\tok\t{median_ns}\t{iterations}",
                    case.name,
                    case.input.len(),
                    parser.name(),
                ),
                Err(error) => println!(
                    "{}\t{}\t{}\t{}\t\t",
                    case.name,
                    case.input.len(),
                    parser.name(),
                    error.replace('\t', " ").replace('\n', " "),
                ),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use markup5ever_rcdom::{Handle, NodeData};

    use super::*;

    fn astral_parent(input: &str, id: &str) -> String {
        #[cfg(feature = "resilient")]
        let document =
            astral_html::Document::parse_recovering(input).expect("valid resource limits");
        #[cfg(not(feature = "resilient"))]
        let document = astral_html::Document::parse(input).expect("valid resource limits");
        document
            .elements()
            .find(|element| {
                element
                    .attribute("id")
                    .is_some_and(|attribute| attribute.value() == id)
            })
            .and_then(|element| element.parent())
            .map(|parent| parent.name().to_owned())
            .expect("element and parent exist")
    }

    fn servo_element(input: &str, id: &str) -> (String, String) {
        fn walk(node: &Handle, parent: &str, id: &str) -> Option<(String, String)> {
            if let NodeData::Element { name, attrs, .. } = &node.data {
                if attrs.borrow().iter().any(|attribute| {
                    attribute.name.local.as_ref() == "id" && attribute.value.as_ref() == id
                }) {
                    return Some((parent.to_owned(), name.ns.to_string()));
                }
                for child in node.children.borrow().iter() {
                    if let Some(result) = walk(child, name.local.as_ref(), id) {
                        return Some(result);
                    }
                }
            } else {
                for child in node.children.borrow().iter() {
                    if let Some(result) = walk(child, parent, id) {
                        return Some(result);
                    }
                }
            }
            None
        }

        let document: RcDom =
            html5ever::parse_document(RcDom::default(), ParseOpts::default()).one(input);
        walk(&document.document, "#document", id).expect("identified Servo element exists")
    }

    #[test]
    fn optional_paragraph_and_list_closing() {
        let paragraphs = "<body><p id=first>one<p id=second>two</body>";
        let list = "<body><ul><li id=first>one<li id=second>two</ul></body>";

        assert_eq!(servo_element(paragraphs, "second").0, "body");
        assert_eq!(servo_element(list, "second").0, "ul");
        #[cfg(feature = "resilient")]
        {
            assert_eq!(astral_parent(paragraphs, "second"), "body");
            assert_eq!(astral_parent(list, "second"), "ul");
        }
        #[cfg(not(feature = "resilient"))]
        {
            assert_eq!(astral_parent(paragraphs, "second"), "p");
            assert_eq!(astral_parent(list, "second"), "li");
        }
    }

    #[test]
    fn table_foster_parenting_is_a_remaining_difference() {
        let input = "<body><table><span id=stray>outside</span><tr><td>inside</table></body>";
        assert_eq!(servo_element(input, "stray").0, "body");
        assert_eq!(astral_parent(input, "stray"), "table");
    }

    #[test]
    fn servo_preserves_foreign_namespace() {
        let input = "<body><svg><g id=vector /></svg></body>";
        assert_eq!(
            servo_element(input, "vector"),
            ("svg".to_owned(), "http://www.w3.org/2000/svg".to_owned())
        );
        assert_eq!(astral_parent(input, "vector"), "svg");
    }
}
