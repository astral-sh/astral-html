#![no_main]

use astral_html::{Document, Limits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let Ok(source) = std::str::from_utf8(bytes) else {
        return;
    };
    let selector = source.as_bytes().first().copied().unwrap_or_default();
    let limits = Limits {
        max_input_bytes: if selector & 1 == 0 { 16_384 } else { 128 },
        max_nodes: if selector & 2 == 0 { 4_096 } else { 16 },
        max_depth: if selector & 4 == 0 { 128 } else { 4 },
    };
    if let Ok(document) = Document::parse_with_limits(source, limits) {
        let count = document.elements().count();
        assert!(count <= source.len() + 1);
        assert!(count <= limits.max_nodes);
        assert!(source.len() <= limits.max_input_bytes);
        // Bound repeated subtree queries independently of the parser's limits.
        for element in document.elements().take(16) {
            assert!(element.is(element.name()));
            assert!(element.descendants().count() <= count);
            assert!(element.children().count() <= count);
            for name in ["href", "data-requires-python", "data-yanked", "missing"] {
                assert_eq!(
                    element.attribute(name).is_some(),
                    element.has_attribute(name)
                );
                std::hint::black_box(element.attribute(name));
            }
            std::hint::black_box(element.text());
        }
    }
});
