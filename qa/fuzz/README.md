# Fuzzing — Mathematical Transformations

This directory contains the fuzzers for the mathematical transformation layer of `NextTabletDriver`.

The fuzzers test coordinate transformation functions with a very large number of automatically generated inputs in order to detect unexpected behavior, panics, non-finite values, and violations of mathematical invariants.

## Why use fuzzing?

Unit tests verify known cases.

Benchmarks measure performance.

Fuzzing answers a different question:

> What happens when the function receives inputs that the developer did not think about?

The driver's mathematical functions primarily operate on `f32` values. They can therefore encounter special values such as:

* `0.0`
* `-0.0`
* `NaN`
* `+∞`
* `-∞`
* extremely large values
* extremely small values
* negative rotations
* rotations greater than `360°`
* zero dimensions
* unusual parameter combinations

The goal is to automatically discover inputs that may reveal a problem.

---

## Architecture

The fuzzers are located in:

```text
fuzz/
├── Cargo.toml
├── README.md
└── fuzz_targets/
    ├── rotate_point.rs
    ├── physical_to_normalized.rs
    ├── normalized_to_screen.rs
    └── apply_relative_delta.rs
```

Each file corresponds to one mathematical function in the driver:

| Fuzzer                   | Tested function            |
| ------------------------ | -------------------------- |
| `rotate_point`           | `rotate_point()`           |
| `physical_to_normalized` | `physical_to_normalized()` |
| `normalized_to_screen`   | `normalized_to_screen()`   |
| `apply_relative_delta`   | `apply_relative_delta()`   |

---

# Installation

Fuzzing uses [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz), which is based on LLVM libFuzzer.

Install `cargo-fuzz`:

```bash
cargo install cargo-fuzz
```

Verify the installation:

```bash
cargo +nightly fuzz --version
```

The fuzzers also use:

```toml
libfuzzer-sys
arbitrary
```

These dependencies are declared in `fuzz/Cargo.toml`.

---

# Running a fuzzer

`cargo +nightly fuzz` commands should be executed from the repository root.

## `rotate_point`

```bash
cargo +nightly fuzz run rotate_point
```

The fuzzer will generate inputs for:

```rust
rotate_point(
    x,
    y,
    center_x,
    center_y,
    rotation_degrees,
)
```

It automatically searches for inputs capable of violating the properties checked by `rotate_point.rs`.

---

## `physical_to_normalized`

```bash
cargo +nightly fuzz run physical_to_normalized
```

Tests:

```rust
physical_to_normalized(
    x_mm,
    y_mm,
    area_x,
    area_y,
    area_w,
    area_h,
    rotation,
)
```

Particular attention is given to zero dimensions and non-finite values.

---

## `normalized_to_screen`

```bash
cargo +nightly fuzz run normalized_to_screen
```

This tests, among other things, the behavior of the `clamp()` operation used to keep normalized coordinates within `[0, 1]`.

---

## `apply_relative_delta`

```bash
cargo +nightly fuzz run apply_relative_delta
```

This tests relative movement, rotation, and sensitivity handling.

---

# Limiting fuzzing duration

By default, a fuzzer can continue indefinitely.

To run a five-minute campaign:

```bash
cargo +nightly fuzz run rotate_point -- -max_total_time=300
```

Ten minutes:

```bash
cargo +nightly fuzz run rotate_point -- -max_total_time=600
```

Longer campaigns are preferable for discovering rare cases.

---

# Limiting the number of executions

The number of test cases can also be limited:

```bash
cargo +nightly fuzz run rotate_point -- -runs=1000000
```

This asks libFuzzer to perform approximately one million executions.

For a quick verification:

```bash
cargo +nightly fuzz run rotate_point -- -runs=10000
```

---

# Running multiple fuzzers

A complete campaign can be run with:

```bash
cargo +nightly fuzz run rotate_point -- -max_total_time=300
cargo +nightly fuzz run physical_to_normalized -- -max_total_time=300
cargo +nightly fuzz run normalized_to_screen -- -max_total_time=300
cargo +nightly fuzz run apply_relative_delta -- -max_total_time=300
```

Each function then receives its own fuzzing campaign.

---

# How does a fuzzer work?

A fuzzer does not simply choose random numbers.

libFuzzer starts with input data and applies different mutation strategies to discover new execution paths.

Conceptually:

```text
        Seed / Corpus
             │
             ▼
       Input generation
             │
             ▼
       Tested function
             │
             ▼
       Property checks
             │
       ┌─────┴─────┐
       │           │
      PASS       FAILURE
       │           │
       ▼           ▼
New input added   Crash
to corpus           │
       │             ▼
       └────────> Reproduction
```

The fuzzer continuously attempts to discover input combinations that have not yet been explored.

---

# Why use `Arbitrary`?

The input structures are declared using:

```rust
#[derive(arbitrary::Arbitrary)]
```

For example:

```rust
#[derive(Debug, arbitrary::Arbitrary)]
struct RotateInput {
    x: f32,
    y: f32,
    center_x: f32,
    center_y: f32,
    rotation_degrees: f32,
}
```

`arbitrary` allows these structures to be constructed automatically from the bytes generated by libFuzzer.

This allows the fuzzer to work directly with the function's parameters rather than requiring manually encoded input data.

---

# Tested properties

The fuzzing targets do not only check whether the functions crash.

They also contain mathematical properties.

## Zero-degree rotation

A rotation of `0°` must preserve the point:

```text
rotate_point(P, C, 0°) = P
```

The fuzzer therefore checks:

```rust
assert_eq!(rx, x);
assert_eq!(ry, y);
```

---

## Rotation around the center

If the point is exactly the rotation center:

```text
P = C
```

then:

```text
rotate(P, C, θ) = P
```

for any valid rotation.

---

## Active-area center

Without rotation, the center of the physical active area must map to the center of normalized coordinates:

```text
physical center → (0.5, 0.5)
```

---

## Screen projection

`normalized_to_screen()` uses `clamp()`.

A normalized coordinate below `0` must be projected onto the corresponding edge.

A normalized coordinate above `1` must be projected onto the opposite edge.

This protects against regressions in:

```rust
u.clamp(0.0, 1.0)
v.clamp(0.0, 1.0)
```

---

## Zero movement

When:

```text
current position = previous position
```

the resulting movement must be:

```text
(dx, dy) = (0, 0)
```

---

# What to do when a fuzzer finds a problem

libFuzzer will generally display a message similar to:

```text
==ERROR: libFuzzer: deadly signal
```

or:

```text
Assertion failed
```

It will also save the input that caused the failure to the fuzzer's artifact directory.

For example:

```text
fuzz/artifacts/rotate_point/crash-xxxxxxxxxxxxxxxx
```

This input is extremely important.

It provides a **deterministic reproduction case**.

---

# Reproducing a crash

If libFuzzer produces:

```text
fuzz/artifacts/rotate_point/crash-abcdef
```

the exact input can be executed again with:

```bash
cargo +nightly fuzz run rotate_point fuzz/artifacts/rotate_point/crash-abcdef
```

The issue can then be analyzed and fixed without relying on randomness.

---

# Corpus

Interesting inputs can be preserved in:

```text
fuzz/corpus/
```

For example:

```text
fuzz/
├── corpus/
│   ├── rotate_point/
│   ├── physical_to_normalized/
│   ├── normalized_to_screen/
│   └── apply_relative_delta/
└── fuzz_targets/
```

The corpus allows libFuzzer to start future campaigns from inputs that have already proven useful.

Over time, the corpus can become a collection of difficult cases discovered during development.

---

# Fuzzing and conventional tests

Fuzzing does not replace unit tests.

Each tool has a different purpose:

| Tool               | Purpose                                     |
| ------------------ | ------------------------------------------- |
| `cargo test`       | Verify known behavior                       |
| Property testing   | Verify general properties                   |
| `cargo +nightly fuzz`       | Automatically search for problematic inputs |
| `cargo bench`      | Measure performance                         |
| Pipeline benchmark | Measure real processing cost                |

A robust implementation should ideally benefit from several of these validation layers.

---

# Fuzzing vs. benchmarks

The benchmarks in `benches/transforms.rs` answer:

> How long does this function take?

The fuzzers answer:

> Is there an unusual input that reveals a problem?

These are independent concerns.

A function can be:

```text
very fast + incorrect
```

or:

```text
correct + too slow
```

Both systems should therefore be maintained.

---

# Recommended development workflow

Before making an important change to `transform.rs`:

```bash
cargo test
```

Then:

```bash
cargo bench --bench transforms
```

After the modification:

```bash
cargo test
cargo bench --bench transforms
```

Then perform a fuzzing campaign:

```bash
cargo +nightly fuzz run rotate_point -- -max_total_time=300
cargo +nightly fuzz run physical_to_normalized -- -max_total_time=300
cargo +nightly fuzz run normalized_to_screen -- -max_total_time=300
cargo +nightly fuzz run apply_relative_delta -- -max_total_time=300
```

For CI or release validation, use a duration appropriate for the available execution time.

---

# Important principle

Fuzzing should not be used to hide an undefined contract.

Before adding a new assertion, determine whether the observed behavior is actually incorrect.

For example, `NaN` or `∞` can be mathematically valid results when the input itself contains `NaN`, `∞`, or causes a division by zero.

The contract of each function must therefore distinguish between:

```text
Valid input
    ↓
Expected behavior

Degenerate input
    ↓
Explicitly defined behavior

Invalid input
    ↓
Documented behavior
```

The fuzzer's role is then to verify that the implementation respects this contract.

---

# Goal

The mathematical layer should be protected by multiple validation levels:

```text
                 transform.rs
                      │
        ┌─────────────┼─────────────┐
        │             │             │
   Unit tests    Property tests   Fuzzing
        │             │             │
        └─────────────┼─────────────┘
                      │
                  Correctness
                      │
                      ▼
                  Benchmarks
                      │
                      ▼
                  Performance
```

This separation makes it possible to detect functional regressions and performance regressions without confusing the two problems.