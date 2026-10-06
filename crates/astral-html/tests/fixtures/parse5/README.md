# parse5 whole-page fixtures

These nine HTML files are copied byte-for-byte from
[inikulin/parse5](https://github.com/inikulin/parse5/tree/e65eae9a9dc27f1b7eb71868d245ca83070ccc4a/test/data),
revision `e65eae9a9dc27f1b7eb71868d245ca83070ccc4a`, also recorded in `REVISION`.
The upstream MIT license is included in `LICENSE`.

| Local file | Upstream path | Bytes |
| --- | --- | ---: |
| `cern.html` | `test/data/location-info/cern/data.html` | 18,262 |
| `dx.html` | `test/data/location-info/dx/data.html` | 99,086 |
| `github-parse5.html` | `test/data/location-info/github-parse5/data.html` | 45,233 |
| `whatwg-html.html` | `test/data/location-info/whatwg-html/data.html` | 143,378 |
| `wiki-42.html` | `test/data/location-info/wiki-42/data.html` | 222,328 |
| `huge-page.html` | `test/data/huge-page/huge-page.html` | 907,953 |
| `lhc.html` | `test/data/sax/lhc/src.html` | 34,018 |
| `nodejsorg.html` | `test/data/sax/nodejsorg/src.html` | 7,639 |
| `npmorg.html` | `test/data/sax/npmorg/src.html` | 10,774 |

`tests/parse5.rs` replays every complete page through `Tokenizer`, `Reader`, and
`Document`, without the fuzz campaigns' 16 KiB cutoff. Each page has a separate
test and an exact byte-count assertion. No network access or JavaScript runtime
is required to run the tests.

The largest page exceeds the default 256-level lexical nesting limit. Its test
asserts that default rejection, then replays it with a 10,000-level depth limit.
The other eight pages use the default limits; no page is truncated or skipped.

Token values are compared against html5gum 0.8.4, pinned as a development-only
dependency. The explicit tokenizer comparison starts in the data state. For
the reader comparison, the oracle selects HTML text states using html5gum's
mapping, with scripting disabled and its `noframe` spelling corrected to
`noframes`. Both readers omit browser tree construction and namespace changes.
Adjacent character tokens are joined and attribute order is ignored; comments,
doctypes, tag names, decoded attributes, text, and start-tag self-closing flags
must agree. Parse-error diagnostics and raw attribute spelling are not compared.

The document comparison checks all retained element names and decoded attributes
in source order against the independent reader tokens. Reader progress, UTF-8
positions, and permanent EOF are checked as well. These are whole-page replay
tests, not comparisons with parse5's browser DOM or its source-location outputs.

None of the pinned pages contains `<?`. The harness asserts this prerequisite
because html5gum 0.8.4 predates processing instructions; an unsupported fixture
fails instead of being skipped. Processing instructions remain covered by the
html5lib suite.

Update all nine pages, their byte counts, the license, and `REVISION` together.
