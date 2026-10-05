# astral-html

A high-performance HTML parser designed for document traversal.

`Reader` emits HTML events, `Tokenizer` accepts explicit fragment context, and `Document` provides immutable element, attribute, and text views. Unchanged strings borrow from the UTF-8 input. Document construction accepts configurable input, node, and nesting limits.

The conformance target is WHATWG tokenization, not browser tree construction. See the [repository](https://github.com/astral-sh/astral-html) for conformance, uv integration, fuzzing, and benchmarks.

Licensed under MIT OR Apache-2.0. The generated character-reference data retains the [CPython license](licenses/CPython.txt).
