# Benchmark fixtures

`iniconfig.html` is the `PYPI_SIMPLE` fixture from
[`astral-tl` 0.8.0's benchmark](https://github.com/astral-sh/astral-tl/blob/04bbd94a054e164a3d057160977197ca25194bc5/benches/tl.rs),
a PyPI Simple API response containing 16 distribution links. Its MIT license is
included alongside it.

The benchmark also uses the pinned uv fixtures in `tests/fixtures/uv` and
generates root and project indexes in memory. Generated indexes are identified
separately from captured responses in the results.
