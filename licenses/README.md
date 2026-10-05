# Third-party data

`crates/astral-html/src/entities_data.rs` is generated directly from WHATWG's
[named character references](https://html.spec.whatwg.org/entities.json). The
source checksum is pinned in `scripts/generate_entities.py`.

To regenerate the checked-in table:

```console
curl -o /tmp/entities.json https://html.spec.whatwg.org/entities.json
python3 scripts/generate_entities.py /tmp/entities.json
```

Test fixtures carry their own provenance and license notices alongside the data.
