# Golden files

Real captures of tablets, replayed through the report parsers. Any change of a parser's output,
even by one byte, fails `cargo test --test golden`.

Each folder is `<vid>-<pid>/` (hexadecimal) and holds:

- `capture.ntdcap`: a capture written by `tools/ntd-recorder`, trimmed to a few dozen reports per
  step so it stays small.
- `snapshot.txt`: what the parser produced for each report, committed. The first line names the
  parser class the tablet's configuration selects.

Captures are real and are never invented or edited by hand.

## Add a tablet

```bash
cargo run --release --manifest-path tools/ntd-recorder/Cargo.toml -- record 28bd:0914
cargo run --release --manifest-path tools/ntd-recorder/Cargo.toml -- trim 28bd-0914.ntdcap qa/golden/28bd-0914/capture.ntdcap
UPDATE_GOLDEN=1 cargo test --test golden
```

Then read `snapshot.txt`: the positions, pressure and buttons must look like what you did during
the capture. Commit both files.

## After a deliberate parser change

Run `UPDATE_GOLDEN=1 cargo test --test golden`, then review the diff of every `snapshot.txt`. A
changed line must be explained by your change.

Tablets without a capture are listed in `MISSING.md`.
