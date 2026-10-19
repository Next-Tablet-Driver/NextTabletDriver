# Fuzzing

A *fuzzer* is a program that calls a function again and again with inputs that it makes up, and watches for crashes and broken rules. It finds the cases that nobody thought of.

This folder has fuzzers for the math that maps tablet coordinates to the screen (`src/core/math/transform.rs`). They use [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz), which is built on LLVM's libFuzzer. A fuzzer is smarter than a random number generator: it watches which parts of the code run, and keeps the inputs that reach new parts.

## What each fuzzer checks

Each fuzzer is a file in [`fuzz_targets/`](fuzz_targets). Besides "it must not crash", it checks rules that must always be true.

| Fuzzer | Function | Rules it checks |
| --- | --- | --- |
| `rotate_point` | `rotate_point` | The result is finite. A rotation of 0° returns the same point. The rotation center does not move. The distance to the center does not change. |
| `physical_to_normalized` | `physical_to_normalized` | The result is finite. Without rotation, the center of the active area maps to (0.5, 0.5). |
| `normalized_to_screen` | `normalized_to_screen` | The result is finite. A value below 0 is clamped to the start of the target area, and a value above 1 to its far edge. |
| `apply_relative_delta` | `apply_relative_delta` | The result is finite. No movement gives no delta. Without rotation, the sensitivity scales the movement exactly. |

The inputs are always **finite numbers in a realistic range** (for example coordinates within ±1000 mm and rotations within ±360°). The fuzzers do not send `NaN` or infinity.

## Set up

You need a nightly Rust toolchain and `cargo-fuzz`:

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
```

The fuzz project is in `qa/fuzz`, not in the default `fuzz/` folder. Always add `--fuzz-dir qa/fuzz`, and run the commands from the root of the repository.

On **Windows**, the fuzzer needs the AddressSanitizer runtime DLL of Visual Studio. Add its folder to your `PATH` first. In PowerShell:

```powershell
$asan = (Get-ChildItem "C:\Program Files\Microsoft Visual Studio\2022\*\VC\Tools\MSVC\*\bin\Hostx64\x64\clang_rt.asan_dynamic-x86_64.dll" | Select-Object -First 1).DirectoryName
$env:Path = "$asan;$env:Path"
```

If you start the fuzzer without it, it stops at once with the error code `0xc0000135` (`STATUS_DLL_NOT_FOUND`). Linux and macOS need nothing extra.

## Run a fuzzer

```bash
cargo +nightly fuzz list --fuzz-dir qa/fuzz                       # the available fuzzers
cargo +nightly fuzz run rotate_point --fuzz-dir qa/fuzz           # run until you press Ctrl+C
```

A fuzzer runs forever by default. Add limits after a `--`:

```bash
cargo +nightly fuzz run rotate_point --fuzz-dir qa/fuzz -- -max_total_time=300   # five minutes
cargo +nightly fuzz run rotate_point --fuzz-dir qa/fuzz -- -runs=100000          # 100000 inputs
```

A longer run finds rarer problems. To run all four for five minutes each:

```bash
for target in rotate_point physical_to_normalized normalized_to_screen apply_relative_delta; do
  cargo +nightly fuzz run "$target" --fuzz-dir qa/fuzz -- -max_total_time=300
done
```

The fuzzers do **not** run in the CI. Run them yourself when you change `src/core/math/transform.rs`, together with the property tests (`cargo nextest run`) and the [benchmarks](../benches/README.md).

## When a fuzzer finds a problem

The fuzzer stops, prints an error such as `Assertion failed` or `deadly signal`, and saves the input that caused it:

```text
qa/fuzz/artifacts/rotate_point/crash-<hash>
```

This file is a reproduction that never changes. Run it again with:

```bash
cargo +nightly fuzz run rotate_point --fuzz-dir qa/fuzz qa/fuzz/artifacts/rotate_point/crash-<hash>
```

Then:

1. Decide if the **function** is wrong, or if the **rule** in the fuzzer is wrong. A rule must state something that is really promised. Do not add a rule only to see the fuzzer fail.
2. Fix the code (or the rule).
3. Copy the failing input into a normal test, in `qa/tests` or next to the function, so the problem cannot come back unseen.
4. Do not commit the `crash-*` files. The `artifacts` folder is not tracked.

## The corpus

[`corpus/<fuzzer>/`](corpus) keeps the inputs that the fuzzer found useful. It is committed, so a new run starts where the last one stopped, instead of from zero. After a long run you may commit new files that reached new code, but keep the folder small.

## Add a fuzzer

1. Create `fuzz_targets/<name>.rs`. Use [`rotate_point.rs`](fuzz_targets/rotate_point.rs) as a model: a struct that implements `Arbitrary`, built with the helpers in [`common.rs`](fuzz_targets/common.rs), and a `fuzz_target!` that calls the function and asserts the rules.
2. Add a `[[bin]]` entry for it in `Cargo.toml`, like the existing ones, with `test = false`, `doc = false` and `bench = false`.
3. Add a row to the table above.

## Fuzzing, tests and benchmarks

| Tool | Question it answers |
| --- | --- |
| Unit tests | Does it give the right answer for the cases I know? |
| Property tests (`qa/tests`) | Does a rule hold for thousands of random inputs? |
| Fuzzing | Is there a strange input that breaks a rule or crashes? |
| [Benchmarks](../benches/README.md) | How fast is it? |

They answer different questions, so use them together.
