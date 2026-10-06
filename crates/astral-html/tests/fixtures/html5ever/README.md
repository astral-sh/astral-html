# html5ever tokenizer fixtures

These files are copied without modification from html5ever's
[`rcdom/custom-html5lib-tokenizer-tests`](https://github.com/servo/html5ever/tree/7760920edff08e6dfd4b62affee39d8084e9dc45/rcdom/custom-html5lib-tokenizer-tests),
revision `7760920edff08e6dfd4b62affee39d8084e9dc45`.
`REVISION` records that upstream commit. The repository's MIT and Apache-2.0
licenses are included alongside the fixtures.

The two files contain 14 cases, expanded to 20 runs across their initial
tokenizer states. They cover duplicate attributes, carriage-return handling,
incomplete tags and character references, and bogus comments after text-mode
end tags. These are html5ever's additional regression tests, separate from its
shared html5lib corpus.

`tests/html5lib.rs` uses the same token-output comparison as the html5lib suite.
It asserts the exact file and run counts and permits no exclusions or expected
failures. Parse-error diagnostics are not compared because astral-html does not
expose them. This suite tests tokenization, not browser tree construction.

Update both fixture files, the licenses, and `REVISION` together. Review changes
to the exact coverage counts in `tests/html5lib.rs`.
