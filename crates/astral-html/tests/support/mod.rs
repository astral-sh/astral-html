use astral_html::Document;

/// The attributes consumed by `uv-client`, in lookup order.
const ATTRIBUTES: [&str; 6] = [
    "data-requires-python",
    "data-core-metadata",
    "data-dist-info-metadata",
    "data-yanked",
    "data-size",
    "data-upload-time",
];

/// Parsed HTML fields at uv's parser boundary.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Index {
    /// The first base URL before any link.
    pub base: Option<String>,
    /// Project status metadata found below the first head element.
    pub status: Vec<(String, Option<String>)>,
    /// Anchors with nonempty hrefs in source order.
    pub links: Vec<Link>,
}

/// The fields uv reads from an anchor.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Link {
    /// Decoded download URL or project URL.
    pub href: String,
    /// `None` is absent; `Some(None)` is a boolean attribute.
    pub attributes: [Option<Option<String>>; 6],
    /// Root index project name, when requested.
    pub text: Option<String>,
}

/// Read the HTML fields that uv needs, without URL or packaging validation.
pub(crate) fn astral(input: &str, root_index: bool) -> Index {
    let document = Document::parse(input).expect("fixture is within parser limits");
    let status = document
        .elements()
        .find(|element| element.is("head"))
        .map(|head| {
            head.descendants()
                .filter(|element| element.is("meta"))
                .filter_map(|meta| {
                    let name = meta.attribute("name")?.value.as_ref();
                    matches!(name, "pypi:project-status" | "pypi:project-status-reason").then(
                        || {
                            (
                                name.to_owned(),
                                meta.attribute("content").map(|attr| attr.value.to_string()),
                            )
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let base = document
        .elements()
        .take_while(|element| !element.is("a") && !element.is("link"))
        .find(|element| element.is("base"))
        .and_then(|base| base.attribute("href").map(|attr| attr.value.to_string()));
    let links = document
        .elements()
        .filter(|element| element.is("a"))
        .filter_map(|anchor| {
            let href = anchor.attribute("href")?.value.as_ref();
            if href.is_empty() {
                return None;
            }
            Some(Link {
                href: href.to_owned(),
                attributes: if root_index {
                    std::array::from_fn(|_| None)
                } else {
                    ATTRIBUTES.map(|name| {
                        anchor
                            .attribute(name)
                            .map(|attr| attr.raw_value.map(|_| attr.value.to_string()))
                    })
                },
                text: root_index.then(|| anchor.text().trim().to_owned()),
            })
        })
        .collect();
    Index {
        base,
        status,
        links,
    }
}

/// Perform the same reads through astral-tl 0.8.0, as used by uv.
pub(crate) fn baseline(input: &str, root_index: bool) -> Index {
    let dom = tl::parse(input, tl::ParserOptions::default()).expect("valid fixture size");
    let status = dom
        .nodes()
        .iter()
        .find(|node| {
            node.as_tag()
                .is_some_and(|tag| tag.name().as_bytes().eq_ignore_ascii_case(b"head"))
        })
        .and_then(|head| head.children())
        .map(|children| {
            children
                .all(dom.parser())
                .iter()
                .filter_map(tl::Node::as_tag)
                .filter(|tag| tag.name().as_bytes().eq_ignore_ascii_case(b"meta"))
                .filter_map(|meta| {
                    let name = meta.attributes().get("name")??.as_utf8_str();
                    matches!(
                        name.as_ref(),
                        "pypi:project-status" | "pypi:project-status-reason"
                    )
                    .then(|| {
                        (
                            name.into_owned(),
                            meta.attributes()
                                .get("content")
                                .flatten()
                                .map(|value| value.as_utf8_str().into_owned()),
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let base = dom
        .nodes()
        .iter()
        .filter_map(tl::Node::as_tag)
        .take_while(|tag| {
            !tag.name().as_bytes().eq_ignore_ascii_case(b"a")
                && !tag.name().as_bytes().eq_ignore_ascii_case(b"link")
        })
        .find(|tag| tag.name().as_bytes().eq_ignore_ascii_case(b"base"))
        .and_then(|tag| tag.attributes().get("href").flatten())
        .map(|value| value.as_utf8_str().into_owned());
    let links = dom
        .nodes()
        .iter()
        .filter_map(tl::Node::as_tag)
        .filter(|tag| tag.name().as_bytes().eq_ignore_ascii_case(b"a"))
        .filter_map(|anchor| {
            let href = anchor.attributes().get("href")??.as_utf8_str();
            if href.is_empty() {
                return None;
            }
            Some(Link {
                href: html_escape::decode_html_entities(&href).into_owned(),
                attributes: if root_index {
                    std::array::from_fn(|_| None)
                } else {
                    ATTRIBUTES.map(|name| {
                        anchor.attributes().get(name).map(|value| {
                            value.map(|value| {
                                html_escape::decode_html_entities(&value.as_utf8_str()).into_owned()
                            })
                        })
                    })
                },
                text: root_index.then(|| anchor.inner_text(dom.parser()).trim().to_owned()),
            })
        })
        .collect();
    Index {
        base,
        status,
        links,
    }
}
