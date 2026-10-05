# Third-party data

`crates/astral-html/src/entities_data.rs` is generated from the WHATWG named
character references in
[CPython v3.14.0](https://github.com/python/cpython/blob/v3.14.0/Lib/html/entities.py).
The source checksum is pinned in `scripts/generate_entities.py`; its license is
preserved in [CPython.txt](CPython.txt). The generator parses the Python literal
without executing the source.

The parser code is licensed under MIT OR Apache-2.0. Test fixtures carry their
own provenance and license notices alongside the data.
