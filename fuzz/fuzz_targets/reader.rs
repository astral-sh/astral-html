#![no_main]

use astral_html::Reader;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let Ok(source) = std::str::from_utf8(bytes) else {
        return;
    };
    let mut reader = Reader::new(source);
    for (index, token) in reader.by_ref().enumerate() {
        assert!(index <= source.len(), "reader must make progress");
        std::hint::black_box(token);
    }
    assert!(reader.next().is_none(), "end of input must be permanent");
});
