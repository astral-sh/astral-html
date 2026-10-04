# astral-html

A read-only HTML parser for package indexes, written in Rust.

`Reader` emits HTML events, `Tokenizer` accepts explicit fragment context, and `Document` provides immutable element, attribute, and text views. Unchanged strings borrow from the UTF-8 input. Document construction accepts configurable input, node, nesting, and attribute limits.

The conformance target is WHATWG tokenization, not browser tree construction. See the [repository](https://github.com/viarius-experiments/astral-html) for conformance, uv integration, fuzzing, and benchmarks.

Licensed under MIT OR Apache-2.0. The generated character-reference data retains the [CPython license](licenses/CPython.txt).
