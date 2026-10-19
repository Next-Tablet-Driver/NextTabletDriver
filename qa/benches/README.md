# Benchmarks

These benchmarks measure how much CPU time the driver spends on each pen packet. A tablet sends up to a thousand packets per second, so a small cost per packet adds up, and a slow packet adds delay for the user. The benchmarks help us notice when a change makes the driver slower.

They use [Criterion](https://bheisler.github.io/criterion.rs/book/), which runs each test many times and reports the average and how much it varies.

## Run them

From the root of the repository:

```bash
cargo bench                       # all of them
cargo bench --bench pipeline      # one file
cargo bench --bench pipeline -- pipeline/absolute   # only the names that contain this text
```

Benchmarks use the optimized release profile (`opt-level = 3`, LTO, one codegen unit). Debug builds give numbers that mean nothing. Criterion writes an HTML report to `target/criterion/report/index.html`.

Close other heavy programs while you measure, and compare numbers from the same computer only. These benchmarks do not run in the CI, because shared CI machines are too noisy.

## What is measured

| File | What it measures |
| --- | --- |
| [`pipeline.rs`](pipeline.rs) | `Pipeline::process()`: the full path of one packet through the transformer, normalizer, pressure curve, filters and projector. It runs in absolute and relative mode, with a cold and a warm cache, and with different pressure curves. This is the main result. |
| [`transforms.rs`](transforms.rs) | The small math functions in `core::math::transform`: `rotate_point`, `physical_to_normalized`, `normalized_to_screen`, `apply_relative_delta`. They show *which* function changed when the pipeline result changes. |
| [`hot_path.rs`](hot_path.rs) | What the polling thread does for one packet, except the USB read, the operating system call and the shared-memory publication: parse, pipeline, a no-op injector, shared state, statistics and the interface channel. It also prints a table of slow packets (p50, p99, p99.9 and max), because an average hides the rare late packet. |

`filters.rs` is an empty placeholder. There is no benchmark for the individual filters yet.

Names describe the situation, not the code: `pipeline/absolute/warm/no_filters/linear`. Keep that pattern when you add a benchmark, because names become the history that you compare against.

The write-up of a real measurement, with its method and its limits, is in [`RESULTS_hot_path.md`](RESULTS_hot_path.md). The raw outputs are in [`raw/`](raw).

## How to read a result

- These are **CPU costs of the driver's own work**, in nanoseconds per packet. They are not input latency. The tablet's report rate, the USB transfer, the operating system and the display add far more, and nothing here measures them.
- Lower is not automatically better. A change that is faster on average but sometimes very slow is worse for a pen. Look at the spread and at the slow packets too.
- Be careful with a result that looks too good. It can be a real gain, a different workload, or a benchmark that no longer measures the real work.
- The end-to-end `pipeline` result decides. A 10 % gain in a small function is only a gain if the whole packet gets faster.

## Rules for a good benchmark

1. **Measure what runs in production.** Call the real functions with realistic data: positions at the center, edges and corners, small and large moves, different rotations and pressures. Do not only test zeros.
2. **Keep setup outside the timed part.** Build the configuration, the pipeline and the caches first. Use Criterion's `iter_batched` when each run needs fresh state.
3. **Test cold and warm.** The first call (it fills caches) and the steady state are different questions.
4. **Stop the compiler from skipping the work.** Pass inputs and results through `std::hint::black_box`.
5. **Do not trade correctness for speed.** Run the tests before and after. A faster wrong answer is a bug.
6. **Do not optimize without a measurement.** Rare paths such as start-up or settings parsing do not need benchmarks unless a profile shows a problem.

## When you change the hot path

These files run for every packet. Take extra care, and run the benchmarks before and after:

- `src/engine/pipeline/` (`mod.rs`, `transformer.rs`, `normalizer.rs`, `projector.rs`)
- `src/core/math/`
- `src/filters/mod.rs`
- `src/engine/tablet_manager/polling.rs`

Follow these steps:

1. Run the benchmarks and save the numbers (a baseline).
2. Make the smallest change that could help.
3. Run the tests, including the property tests (`qa/tests`) and, for the math, the [fuzzers](../fuzz/README.md).
4. Run the benchmarks again and compare. Run three times if the difference is small.
5. Keep the change only if the whole packet got faster and the code is not much harder to read. Say in the pull request what became faster, why, and how you know the behavior did not change.

Other tools answer other questions: a flamegraph shows where time goes, and `cargo bloat` shows what makes the binary big. Criterion only tells you whether the time changed.
