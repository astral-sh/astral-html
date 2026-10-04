# uv fixtures

These 32 inputs are copied without modification from the 31 tests in
[`uv-client/src/html.rs`](https://github.com/astral-sh/uv/blob/46b84fd0bfec23b72f29e8e2185ba68a65052f48/crates/uv-client/src/html.rs).
Each filename identifies its source test. The `_2` suffix identifies the second
input in `parse_simple_html_case_insensitively`.

The fixtures cover package and root indexes, flat indexes, AWS CodeArtifact,
base URLs, hashes, escaped attributes, project status, case-insensitive names,
and absent or empty link fields. They exercise HTML extraction; URL validation,
hash validation, and Python package semantics remain uv's responsibility.

uv is dual-licensed under MIT and Apache-2.0. Both licenses are included here.
