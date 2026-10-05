# Third-party data

`crates/astral-html/src/entities_data.rs` is generated directly from WHATWG's
[named character references](https://html.spec.whatwg.org/entities.json). The
source checksum is pinned in `scripts/generate_entities.py`. WHATWG licenses
portions of the standard incorporated into source code under BSD-3-Clause; the
notice is retained in [WHATWG.txt](../crates/astral-html/licenses/WHATWG.txt).

To regenerate the checked-in table:

```console
curl -o /tmp/entities.json https://html.spec.whatwg.org/entities.json
python3 scripts/generate_entities.py /tmp/entities.json
```

The parser code is licensed under MIT OR Apache-2.0. Test fixtures carry their
own provenance and license notices alongside the data.
