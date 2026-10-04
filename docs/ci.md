# Linux CI evidence

All ten jobs passed on 2026-10-04 for
[`0819e203f44498a97b02996aed49b81caa032c41`](https://github.com/viarius-experiments/astral-html/commit/0819e203f44498a97b02996aed49b81caa032c41),
the tested revision of [PR #14](https://github.com/viarius-experiments/astral-html/pull/14).

| Workflow | Checks | Result |
| --- | --- | --- |
| [CI](https://github.com/viarius-experiments/astral-html/actions/runs/37208040491) | Rust 1.97.0 and stable tests, doctests and documentation; formatting and Clippy; package verification | All four jobs passed |
| [uv integration](https://github.com/viarius-experiments/astral-html/actions/runs/37208040489) | Apply the parser adapter to pinned uv and run its original `uv-client` HTML tests | Passed |
| [Fuzz](https://github.com/viarius-experiments/astral-html/actions/runs/37208040496) | Five corpus-seeded targets with AddressSanitizer | All five jobs passed |

The fuzz artifacts record checkout
`d080dcc361f9b30e747de4c4c235b322ccedfe9a`, GitHub's temporary merge commit.
Its Git tree is identical to the tested PR head:
`04fbbcd8e2dc8fed59677544891dea07a8b10513`.

## AddressSanitizer results

Each target ran for 31 seconds with 6,747 distinct fixture inputs available.
There were no crashes, timeouts, sanitizer reports, or differential mismatches.

| Target | Executions | Job log |
| --- | ---: | --- |
| `tokenize` | 215,846 | [log](https://github.com/viarius-experiments/astral-html/actions/runs/37208040496/job/111453271925) |
| `entities` | 180,642 | [log](https://github.com/viarius-experiments/astral-html/actions/runs/37208040496/job/111453271960) |
| `reader` | 698,354 | [log](https://github.com/viarius-experiments/astral-html/actions/runs/37208040496/job/111453271977) |
| `document` | 536,205 | [log](https://github.com/viarius-experiments/astral-html/actions/runs/37208040496/job/111453271891) |
| `differential` | 70,607 | [log](https://github.com/viarius-experiments/astral-html/actions/runs/37208040496/job/111453271843) |

The total is 1,701,654 libFuzzer executions, including invalid UTF-8 inputs and
inputs filtered by an oracle. The fixture corpus digest was
`8f85f8345d1883832466771dfac8fbdf174b93a7ba52505c096f583edb5b79d7`.
The import covered 6,827 html5lib inputs, 32 uv fixtures, and 25 handwritten seed
files; four non-scalar inputs were excluded and duplicate strings were combined.

The jobs used `nightly-2026-09-07`, resolving to Rust 1.100.0-nightly
(`5a2be9f5f075d31e3ca5526b5b029881ce441253`, LLVM 23.1.1), on x86-64 Linux.
`cargo-fuzz` 0.13.2 enables AddressSanitizer by default; the workflow also sets
`ASAN_OPTIONS=detect_leaks=1`. The recorded run parameters were seed `2`,
`-max_total_time=30`, `-max_len=16384`, `-len_control=0`, `-timeout=5`, and
`-rss_limit_mb=2048`, using the committed HTML dictionary.

The workflow retains logs, input hashes, generated corpora, and compiler metadata
as artifacts for 30 days. These short sanitizer runs complement the longer
[local fuzz campaigns](fuzzing.md#recorded-local-campaign); they do not establish
sustained fuzz coverage. See [conformance](conformance.md) and
[uv integration](uv.md) for the contracts checked and their exclusions.
