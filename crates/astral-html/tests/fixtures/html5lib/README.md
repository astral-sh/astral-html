# html5lib tokenizer fixtures

These files are copied without modification from
[html5lib/html5lib-tests](https://github.com/html5lib/html5lib-tests/tree/c777c408b61078ea2eb4acefc2535f54dbc8b28a/tokenizer),
revision `c777c408b61078ea2eb4acefc2535f54dbc8b28a` (October 1, 2026).
Their MIT license is included in `LICENSE`; `REVISION` records the upstream commit.
`tokenizer/README.md` is the upstream test format documentation.

The harness runs 7,045 state-expanded cases. It excludes exactly four inputs
containing unpaired UTF-16 surrogates, which cannot be represented by Rust's
`&str`, and the four `xmlViolation.test` cases for optional XML infoset coercion.
The remaining case in `unicodeCharsProblematic.test` is included.

The harness compares every emitted token, normalized tag and attribute name,
decoded attribute value, self-closing flag, document type identifier and quirks
flag. Adjacent character tokens are coalesced as the fixture format requires.
It does not assert parse-error diagnostics or positions, which the public API
does not expose. This is a tokenizer conformance bar, not a tree-construction
conformance claim. The corpus includes the October 2026 processing-instruction
rules; older tokenizer oracles may differ on those inputs.

Update the full directory and `REVISION` together. Review every change to the
exact run and exclusion counts in `tests/html5lib.rs`.
