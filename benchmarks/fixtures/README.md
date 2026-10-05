# HTML corpus

These five documents are vendored inputs, not generated stress cases. The
benchmark reads them from disk and does not fetch network resources. The
657,334 HTML bytes are pinned by source revision and SHA-256 in
[`manifest.json`](manifest.json). Sizes describe the uncompressed UTF-8 files.

| ID | Bytes | Source and workload |
| --- | ---: | --- |
| `iniconfig` | 6,144 | A captured PyPI Simple API response preserved by astral-tl: 16 download links with package metadata attributes. |
| `rust-book` | 88,978 | A published mdBook tutorial chapter: navigation, prose, 38 code blocks, and inline theme scripts. |
| `pep8` | 121,592 | A published Sphinx article: prose, 77 code blocks, cross-references, and inline SVG icons. |
| `pep-index` | 418,744 | The published PEP catalog: 11 tables, 1,128 rows, and 2,099 links. |
| `bootstrap-dashboard` | 21,876 | Bootstrap's published application example: navigation, a report table, a search input, inline styles, and SVG icons. |

The Rust, PEP, and Bootstrap files are rendered HTML committed by their upstream
projects, copied without reformatting or rebuilding. The Bootstrap page is an
application example with sample data, not a captured production dashboard. The
iniconfig file is the exact `PYPI_SIMPLE` Rust raw-string value in the pinned
astral-tl benchmark; it is also identical to this repository's existing
`crates/astral-html/benches/fixtures/iniconfig.html`.

The corpus spans several page structures and sizes, but is not a representative
sample of all websites. Referenced stylesheets, scripts, fonts, and images are
not included or executed. Results describe parsing these HTML responses, not
browser rendering or application execution.

## Provenance and licenses

Each manifest entry links to its exact source file and lists the local license
notices that accompany it. [`licenses/SOURCES.json`](licenses/SOURCES.json)
records the pinned source of every notice.

- **iniconfig:** the fixture is distributed in astral-tl under MIT, copyright
  Timo. Its original capture date is not recorded upstream; the source commit
  identifies the benchmark copy, not a live PyPI response.
- **Rust Book:** the book is MIT OR Apache-2.0; this copy retains its MIT notice.
  The embedded mdBook theme template is covered by MPL-2.0, whose complete text
  is also included. The unmodified HTML is supplied here as source text. The
  [mdBook template source](https://github.com/rust-lang/mdBook/blob/94b922d27aea47183ebf270e2f6f32561d960852/src/theme/index.hbs)
  and [book source](https://github.com/rust-lang/book) are available upstream;
  the rendered-file revision in the manifest identifies the benchmark bytes.
- **PEP 8:** the article's own copyright section places it in the public
  domain. Its notice is retained in the HTML and separately under `licenses/`.
- **PEP index and theme:** the
  [PEP Sphinx extensions license](https://github.com/python/peps/blob/730372f74cd1c200170478fb91a9b3f07a737acd/pep_sphinx_extensions/LICENCE.rst)
  places that directory, including the index generator and page template, in
  the public domain or under CC0-1.0. Both its notice and the complete CC0 text
  are included. This source revision is named in the rendered-pages deployment
  commit.
- **PEP inline icons:** the
  [template](https://github.com/python/peps/blob/730372f74cd1c200170478fb91a9b3f07a737acd/pep_sphinx_extensions/pep_theme/templates/partials/icons.html)
  credits Just the Docs through Furo. Their MIT notices, along with the MIT
  notices for the Feather and Tabler icon designs, are retained.
- **Bootstrap dashboard:** the example HTML is Bootstrap code under MIT; its
  inline Bootstrap Icons are also MIT. Both notices are included. Bootstrap's
  prose documentation has a separate CC BY 3.0 license; this fixture is the
  dashboard example, not a documentation article.

## Reproducing and checking the bytes

A checkout contains everything needed for an offline run. Verify the vendored
HTML and license-file presence from the repository root with Python:

```console
python3 - <<'PY'
import hashlib
import json
from pathlib import Path

root = Path("benchmarks/fixtures")
manifest = json.loads((root / "manifest.json").read_text())
assert manifest["version"] == 1
for fixture in manifest["fixtures"]:
    data = (root / fixture["file"]).read_bytes()
    assert len(data) == fixture["bytes"], fixture["id"]
    assert hashlib.sha256(data).hexdigest() == fixture["sha256"], fixture["id"]
    data.decode("utf-8")
    for notice in fixture["license_files"]:
        assert (root / notice).is_file(), notice
    print(fixture["id"], len(data), "verified")
PY
```

To independently reacquire a file, fetch its pinned GitHub Contents API path,
base64-decode the `content` field, and compare SHA-256 before replacing the local
copy. For example:

```console
gh-auto api 'repos/python/peps/contents/pep-0008/index.html?ref=73849c82abc9b9061664b2e2a064019f80c496db' --jq .content | base64 --decode > /tmp/pep8.html
sha256sum /tmp/pep8.html
```

The `iniconfig` source is a Rust file instead of a standalone HTML file: extract
the bytes between `const PYPI_SIMPLE: &str = r#"` and the following `"#;`, without
adding a newline. All other manifest sources are copied as complete files.
The `pep8-public-domain.txt` notice is the `Copyright` section at the end of
the pinned PEP source; other notices are complete copies of their linked files.
