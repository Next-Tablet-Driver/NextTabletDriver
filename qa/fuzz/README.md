# Fuzzing

A *fuzzer* is a program that calls a function again and again with inputs that it makes up, and watches for crashes and broken rules. It finds the cases that nobody thought of.

This folder has fuzzers for the math that maps tablet coordinates to the screen (`crates/ntd-core/src/math/transform.rs`) and for the HID report parsers of `ntd-tablets`, the code that receives bytes from outside. They use [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz), which is built on LLVM's libFuzzer. A fuzzer is smarter than a random number generator: it watches which parts of the code run, and keeps the inputs that reach new parts.

## What each fuzzer checks

Each fuzzer is a file in [`fuzz_targets/`](fuzz_targets). Besides "it must not crash", it checks rules that must always be true.

| Fuzzer | Function | Rules it checks |
| --- | --- | --- |
| `rotate_point` | `rotate_point` | The result is finite. A rotation of 0° returns the same point. The rotation center does not move. The distance to the center does not change. |
| `physical_to_normalized` | `physical_to_normalized` | The result is finite. Without rotation, the center of the active area maps to (0.5, 0.5). |
| `normalized_to_screen` | `normalized_to_screen` | The result is finite. A value below 0 is clamped to the start of the target area, and a value above 1 to its far edge. |
| `apply_relative_delta` | `apply_relative_delta` | The result is finite. No movement gives no delta. Without rotation, the sensitivity scales the movement exactly. |
| `parser_acepen` | `ntd_tablets::parsers::acepen` | No panic on any bytes; the same bytes give the same result. |
| `parser_bosto` | `ntd_tablets::parsers::bosto` | No panic on any bytes; the same bytes give the same result. |
| `parser_fallback` | `ntd_tablets::parsers::fallback` | No panic on any bytes; the same bytes give the same result. |
| `parser_floogoo` | `ntd_tablets::parsers::floogoo` | No panic on any bytes; the same bytes give the same result. |
| `parser_genius` | `ntd_tablets::parsers::genius` | No panic on any bytes; the same bytes give the same result. |
| `parser_huion` | `ntd_tablets::parsers::huion` | No panic on any bytes; the same bytes give the same result. |
| `parser_lifetec` | `ntd_tablets::parsers::lifetec` | No panic on any bytes; the same bytes give the same result. |
| `parser_robotpen` | `ntd_tablets::parsers::robotpen` | No panic on any bytes; the same bytes give the same result. |
| `parser_skip_byte` | `ntd_tablets::parsers::skip_byte` | No panic on any bytes; the same bytes give the same result. |
| `parser_tenmoon` | `ntd_tablets::parsers::tenmoon` | No panic on any bytes; the same bytes give the same result. |
| `parser_uclogic` | `ntd_tablets::parsers::uclogic` | No panic on any bytes; the same bytes give the same result. |
| `parser_veikk` | `ntd_tablets::parsers::veikk` | No panic on any bytes; the same bytes give the same result. |
| `parser_viewsonic` | `ntd_tablets::parsers::viewsonic` | No panic on any bytes; the same bytes give the same result. |
| `parser_wacom` | `ntd_tablets::parsers::wacom` | No panic on any bytes; the same bytes give the same result. |
| `parser_xencelabs` | `ntd_tablets::parsers::xencelabs` | No panic on any bytes; the same bytes give the same result. |
| `parser_xenx` | `ntd_tablets::parsers::xenx` | No panic on any bytes; the same bytes give the same result. |
| `parser_xp_pen` | `ntd_tablets::parsers::xp_pen` | No panic on any bytes; the same bytes give the same result. |

Each `parser_<family>` fuzzer feeds arbitrary bytes to every parser of one family (`parser_common.rs` finds them: the parser classes the tablet definitions name, mapped by `create_parser`). The positions are not checked against the bounds of the definition, because a pen that is out of range legally reports values beyond them.

The maths inputs are always **finite numbers in a realistic range** (for example coordinates within ±1000 mm and rotations within ±360°). The fuzzers do not send `NaN` or infinity.

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

A longer run finds rarer problems. To run the four maths fuzzers for five minutes each:

```bash
for target in rotate_point physical_to_normalized normalized_to_screen apply_relative_delta; do
  cargo +nightly fuzz run "$target" --fuzz-dir qa/fuzz -- -max_total_time=300
done
```

The `parser_*` fuzzers run in the CI (job `fuzz`, 60 seconds per family, from the committed corpus). The four maths fuzzers do not: run them yourself when you change `crates/ntd-core/src/math/transform.rs`, together with the property tests (`cargo nextest run`) and the [benchmarks](../benches/README.md).

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
3. Copy the failing input into a normal test, in `qa/tests` or next to the function, so the problem cannot come back unseen. For a parser, also add the input to the corpus of its family.
4. Do not commit the `crash-*` files. The `artifacts` folder is not tracked.

## The corpus

[`corpus/<fuzzer>/`](corpus) keeps the inputs that the fuzzer found useful. It is committed, so a new run starts where the last one stopped, instead of from zero. After a long run you may commit new files that reached new code, but keep the folder small.

The parser corpus is seeded with **real captures** only (`tools/ntd-recorder`): `corpus/parser_xp_pen/` holds the reports of `qa/golden/28bd-0914/capture.ntdcap`. Never invent a seed; add the reports of a new golden capture to the corpus of its family (the family is the parser named on the first line of its `snapshot.txt`).

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
