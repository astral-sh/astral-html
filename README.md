# astral-html

A read-only HTML parser for package indexes, written in Rust.

We are building this library to replace `astral-tl` in uv. It will provide HTML tokenization, attribute decoding, and a read-only document view without mutation or serialization APIs.

The conformance target is WHATWG tokenization for UTF-8 input. Browser tree construction, encoding detection, and script execution are outside the scope of this library. We will track conformance, compatibility, fuzzing, and performance evidence in this repository before recommending a migration.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
