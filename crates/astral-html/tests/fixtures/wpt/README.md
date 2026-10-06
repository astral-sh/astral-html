# Selected Web Platform Tests

The fixtures and adaptations in `../../wpt.rs` come from
[web-platform-tests/wpt](https://github.com/web-platform-tests/wpt), pinned to
`3d8a779128cf89fff4732953eff7bbac682b3063` (also recorded in `REVISION`). The
upstream BSD 3-Clause license is retained in `LICENSE.md`.

These six Rust tests exercise **2,252 tokenizer input cases**, with no runtime
downloads or skipped cases. They adapt selected tokenizer behavior, rather than
running WPT's browser harness or claiming full WPT conformance.

All paths below are relative to upstream `html/syntax/parsing/` at the pinned
revision:

| Upstream paths                                                                                                                                            | Cases | Adaptation                                                                                                                                                                                                                                                                                             |
| --------------------------------------------------------------------------------------------------------------------------------------------------------- | ----: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `named-character-references.html`, `named-character-references-data.js`                                                                                   | 2,231 | All entity spellings, including legacy spellings without a semicolon, map to their expected Unicode strings in `named-character-references.json`. Token text replaces the browser's `p.innerHTML` / `textContent` observation.                                                                         |
| `ambiguous-ampersand.html`                                                                                                                                |     2 | The source string is parsed as text and as an `href` attribute. The attribute expectation is WPT's expected `a.href` value with its URL percent-encoding removed; tokenization does not resolve or serialize URLs. The equivalent network and `document.write` executions become one case per context. |
| `newline-normalization-cr-then-lf.html`                                                                                                                   |     6 | Both newline inputs are tested in text and in single- and double-quoted attributes. The `<body>` / `<div>` wrappers and DOM lookup are omitted; the source strings and expected values are unchanged.                                                                                                  |
| `doctype-system-identifier-distinction.html`, `support/doctype-system-id-{empty,missing,nonempty}-{frameset,transitional}.html`, `empty-doctype-ids.html` |     7 | The declarations are copied from the six support files and the empty-ID test. Check token name, public ID, absent versus empty versus nonempty system ID, and the tokenizer's `force_quirks` flag. WPT's `document.compatMode` assertions belong to tree construction and are not imported.            |
| `no-doctype-name.html`, `support/no-doctype-name-{space,line,eof}.html`                                                                                   |     4 | The declarations are copied without the support files' trailing document newlines. Check the absent token name and IDs, plus `force_quirks`. WPT observes an empty DOM name; its title explicitly distinguishes that from the tokenizer's absent name.                                                 |
| `truncated-markup-declaration-at-eof.html`                                                                                                                |     2 | The two distinct inputs and expected comment/text or doctype name are retained. The repeated iframe execution is omitted. Also check the malformed doctype's absent IDs and `force_quirks` flag at the tokenizer level.                                                                                |

The doctype field and flag expectations are tokenizer-contract adaptations of
the upstream inputs, not WPT's browser quirks-mode results. An HTML 4.01 public
identifier can affect the tree builder's mode while the tokenizer's
`force_quirks` flag remains false.

## Named-reference transformation

`named-character-references.json` is a lossless projection of the upstream
JavaScript object's keys and `characters` values. The `codepoints` metadata and
`window.data =` wrapper are omitted. Every upstream entry is included; Unicode
surrogate pairs in JavaScript escape sequences are combined into Unicode scalar
values for UTF-8 JSON.

To reproduce it, fetch the pinned `named-character-references-data.js` as
`/tmp/wpt-named-character-references-data.js`, then run this from this
directory:

```python
import ast
import json
from pathlib import Path
import re

lines = Path('/tmp/wpt-named-character-references-data.js').read_text().splitlines()
assert lines[0] == 'window.data = {' and lines[-1] == '};'
pattern = re.compile(r"  ('(?:[^'\\]|\\.)*'): \{ 'codepoints': \[([^]]+)\], 'characters': ('(?:[^'\\]|\\.)*') \},?")
references = {}
for line in lines[1:-1]:
    match = pattern.fullmatch(line)
    assert match, line
    name = ast.literal_eval(match[1])
    points = ast.literal_eval('[' + match[2] + ']')
    characters = ast.literal_eval(match[3]).encode('utf-16', 'surrogatepass').decode('utf-16')
    assert characters == ''.join(map(chr, points))
    assert name not in references
    references[name] = characters
assert len(references) == 2231
Path('named-character-references.json').write_text(
    json.dumps(references, ensure_ascii=False, indent=2) + '\n'
)
```
