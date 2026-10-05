# Ruff benchmark SVG

[`ruff.svg`](ruff.svg) is the unmodified light-theme SVG used in
[Ruff's README](https://github.com/astral-sh/ruff):

https://user-images.githubusercontent.com/1309177/232603516-4fb4892d-585c-4b20-b810-3db9161831e4.svg

The [original dark asset](https://user-images.githubusercontent.com/1309177/232603514-c95e9b0f-6b31-43de-9a80-9e844173fd6a.svg)
differs only by changing text color from `#333333` to `#C9D1D9`.

Our generator replaces the labels, values, axis scale, and row positions while
retaining the SVG's dimensions, typography, purple bars, and grid styling.
The chart uses astral-html's Reader API; Document results remain in the detailed
benchmark data. Both generated SVGs follow the template's light/dark treatment.

The template and adapted SVGs retain Ruff's [MIT license](LICENSE-RUFF),
copyright 2022 Charles Marsh.
