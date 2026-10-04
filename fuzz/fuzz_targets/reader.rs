#![no_main]

use astral_html::Reader;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let Ok(source) = std::str::from_utf8(bytes) else {
        return;
    };
    let mut reader = Reader::new(source);
    let mut previous = 0;
    while let Some(token) = reader.next() {
        let position = reader.position();
        assert!(position > previous, "each token must consume input");
        assert!(source.is_char_boundary(position));
        previous = position;
        std::hint::black_box(token);
    }
    assert_eq!(reader.position(), source.len());
    assert!(reader.next().is_none(), "end of input must be permanent");
});
