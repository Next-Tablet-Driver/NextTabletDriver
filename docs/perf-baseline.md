# Performance baseline (before the v3 rewrite)

This is the reference every v3 change is compared against (roadmap P0.8). A change that makes the
numbers below worse by more than the tolerance needs a reason in its PR.

> **What is measured.** The CPU time the driver spends on its own work for one pen packet, with a
> fake driver and a no-op input sink. It is **not** input latency: the tablet's report interval,
> the USB read, the operating system and the display add far more and are not measured here.

## Measuring machine

| | |
|---|---|
| CPU | AMD Ryzen 5 5500, 6 cores / 12 threads |
| Memory | 16 GB |
| OS | Windows 11 Pro |
| Rust | `rustc 1.99.1` |
| Profile | `cargo bench` (release, `opt-level = 3`, `lto = true`, `codegen-units = 1`) |
| Code | commit `8f538cb` (branch `chore/v3-phase-0`), 2026-10-31 |

Compare numbers from the same machine only. These benchmarks do not run in CI (shared runners are
too noisy); see [`qa/benches/README.md`](../qa/benches/README.md).

## Tolerance

**5 %.** A result is a regression when its **median over 3 complete runs** is more than 5 % above
the baseline median below. A single run is not enough: one run of `tray_hidden` can differ from the
next by 8 % on this machine, so never judge a change from one run.

Tail latency (p99, p99.9, max) is far noisier than the mean. It is recorded as a range for
information, not as a pass/fail criterion.

## How to measure

```bash
cargo bench --bench hot_path
```

Do it three times, close other heavy programs, and keep the machine otherwise idle. The raw output
of the baseline runs is in [`qa/benches/raw/`](../qa/benches/raw) (`p08_*`).

## Baseline: `hot_path` (the headline)

The production per-packet function, `driver.parse → Pipeline::process → inject (no-op) → publish
shared state → stats → UI channel`. Source: [`qa/benches/hot_path.rs`](../qa/benches/hot_path.rs).

Criterion mean per packet, point estimate of each run, and the median of the three:

| Scenario | Run 1 | Run 2 | Run 3 | **Baseline (median)** |
|---|---:|---:|---:|---:|
| `tray_hidden` (nobody reads the state) | 143.5 ns | 132.1 ns | 132.4 ns | **132.4 ns** |
| `ui_60hz` (UI snapshot at 60 Hz) | 136.6 ns | 139.1 ns | 137.7 ns | **137.7 ns** |
| `ui_stress` (reader in a tight loop, upper bound) | 228.2 ns | 229.3 ns | 231.6 ns | **229.3 ns** |
| `ui_snapshot_capture` (reader side) | 332.0 ns | 331.6 ns | 361.0 ns | **332.0 ns** |

Per-packet tail, in ns, over the three runs (20 000 batches of 64 packets each):

| Scenario | p50 | p99 | p99.9 |
|---|---:|---:|---:|
| `tray_hidden` | 125 | 203 - 370 | 856 - 1247 |
| `ui_60hz` | 123 - 127 | 216 - 295 | 900 - 1077 |
| `ui_stress` | 212 - 216 | 477 - 575 | 1402 - 1973 |

This is consistent with the earlier measurement in
[`qa/benches/RESULTS_hot_path.md`](../qa/benches/RESULTS_hot_path.md) (122 - 126 ns for the first
two scenarios, 218 ns for `ui_stress`), so the baseline did not drift with the rewrite's first
commits.

## `pipeline`: recorded, but not usable as a baseline yet

[`qa/benches/pipeline.rs`](../qa/benches/pipeline.rs) gave, in microseconds per iteration:

| Benchmark | Run 1 | Run 2 | Run 3 |
|---|---:|---:|---:|
| `absolute/cold/no_filters/linear` | 106.7 | 185.2 | 186.8 |
| `absolute/warm/no_filters/linear` | 183.9 | 192.6 | 194.0 |
| `relative/warm/no_filters/linear` | 189.6 | 190.4 | 104.5 |
| `absolute/warm/pressure/linear` | 193.3 | 189.1 | 94.0 |
| `absolute/warm/pressure/exponential` | 185.6 | 189.2 | 110.4 |
| `absolute/warm/pressure/custom_32_points` | 197.9 | 193.7 | 107.7 |

Two things make these numbers unfit for a 5 % tolerance:

1. **They jump between runs** (for example 190 µs, then 94 µs, for the same code), which is far
   beyond the tolerance and beyond what the code explains.
2. **They are three orders of magnitude above `hot_path`** (about 100 - 190 µs against about
   130 ns for a larger piece of work). The benchmark builds a `SharedState` for every iteration
   (`iter_batched`) and the state is dropped inside the timed section, so the figure is dominated by
   that construction and drop, not by `Pipeline::process`.

So `hot_path` is the baseline. Fixing `pipeline.rs` so that it times only `process` (for example
with `BatchSize::PerIteration` and the state dropped outside the timed closure) is a separate task;
until then, do not use the `pipeline` figures to accept or reject a change.
