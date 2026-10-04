#![no_main]

use astral_html::{State, Tokenizer};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let Ok(source) = std::str::from_utf8(bytes) else {
        return;
    };
    let context_end = source
        .char_indices()
        .nth(32)
        .map_or(source.len(), |(offset, _)| offset);
    let context = &source[..context_end];
    for (state, last_start_tag) in [
        (State::Data, None),
        (State::Rcdata, Some("title")),
        (State::Rawtext, Some("style")),
        (State::ScriptData, Some("script")),
        (State::Plaintext, None),
        (State::Cdata, None),
        (State::Rcdata, Some(context)),
        (State::Rawtext, Some(context)),
        (State::ScriptData, Some(context)),
    ] {
        let mut tokenizer = Tokenizer::with_state(source, state, last_start_tag);
        let mut previous = 0;
        while let Some(token) = tokenizer.next() {
            let position = tokenizer.position();
            assert!(position > previous, "each token must consume input");
            assert!(source.is_char_boundary(position));
            previous = position;
            std::hint::black_box(token);
        }
        assert_eq!(tokenizer.position(), source.len());
        assert!(tokenizer.next().is_none(), "end of input must be permanent");
    }
});
