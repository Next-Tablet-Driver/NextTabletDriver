# Per-packet CPU cost of the engine's own bookkeeping (not end-to-end latency)

> **Scope.** Everything below is the CPU time the driver spends on its own work for one
> packet, measured on one machine with a fake driver and a no-op injector. It is **not**
> input latency: the tablet's report interval, the USB/HID read, the OS injection and the
> display are not measured and dominate what a user feels. The improvement is about 185 ns
> per packet, about 0.02 % of a 1 ms report interval. It has not been validated with a real
> tablet.

Benchmark: [`hot_path.rs`](hot_path.rs). It runs the production per-packet function
(`tablet_manager::polling::process_packet`) with a fixed driver and a no-op input sink:

```
driver.parse -> Pipeline::process -> inject (no-op) -> publish shared state -> stats -> UI channel
```

**Not measured:** the HID read, the real OS injection (`SendInput` / uinput) and the
shared-memory publication. The numbers below are therefore the cost of the driver's own
bookkeeping per packet, not end-to-end latency.

Machine: AMD Ryzen 5 5500 (12 threads), Windows 11, rustc 1.99.0, `cargo bench` (release,
`lto = true`, `codegen-units = 1`). Raw outputs of every run are in [`raw/`](raw).

## Method

- 3 complete runs per variant, same machine, same session, nothing else heavy running.
- **Criterion mean** (100 samples, 5 s each) per scenario. The figure below is the median of
  the three runs' point estimates.
- **Tail table**: 20 000 batches of 64 packets, timed per batch (`Instant` has 100 ns
  resolution on Windows, coarser than one packet), reported as ns per packet.
- Scenarios: `tray_hidden` (no reader), `ui_60hz` (a thread snapshots the state and drains
  the channel at 60 Hz, like the real UI), `ui_stress` (a thread snapshots in a tight loop;
  an upper bound on contention, not a realistic load).

## Variants

| Variant | Code |
|---|---|
| **baseline** | `process_packet` as it was: 8 clock reads per packet, blocking `write()` on the stats lock. |
| **optA** | 5 clock reads (each timestamp reused), `try_write()` on the stats lock. |
| **optB** | optA, plus per-stage timings (parse / inject) only measured for the packet that refreshes the statistics (~60 Hz): 2 clock reads on every other packet. |

## Results: mean CPU cost per packet (Criterion)

| Scenario | baseline | optA | optB | optB vs baseline |
|---|---:|---:|---:|---:|
| `tray_hidden` | 306 ns | 217 ns | **122 ns** | **-60 %** |
| `ui_60hz` | 321 ns | 220 ns | **126 ns** | **-61 %** |
| `ui_stress` | 436 ns | 316 ns | **218 ns** | **-50 %** |
| `ui_snapshot_capture` (reader side) | 322 ns | 325 ns | 309 ns | no change (run-to-run noise is about +-10 ns) |

Every Criterion comparison against the previous variant was significant (p < 0.05) with
changes of -29 % (optA) and -44 % (optB) on the first run of each variant; the later runs
of the same variant report no change, as expected.

## Results: p50 per packet (stable across runs)

| Scenario | baseline | optA | optB |
|---|---:|---:|---:|
| `tray_hidden` | 298 ns | 211 ns | 117 ns |
| `ui_60hz` | 303 ns | 211 ns | 117 ns |
| `ui_stress` | 416-505 ns | 292-294 ns | 205-208 ns |

## Results: tail latency (p99 / p99.9, ns per packet, range over the 3 runs)

| Scenario | baseline p99 | optA p99 | optB p99 | baseline p99.9 | optA p99.9 | optB p99.9 |
|---|---:|---:|---:|---:|---:|---:|
| `tray_hidden` | 466-827 | 297-597 | 198-369 | 1205-2109 | 814-1841 | 1027-1730 |
| `ui_60hz` | 639-862 | 419-464 | 188-364 | 1820-2148 | 1533-1691 | 989-1597 |
| `ui_stress` | 759-880 | 544-552 | 345-466 | 1862-1962 | 1244-1392 | 908-1580 |

**Reading the tails honestly:** the ranges overlap between variants. p99 improves in the
same direction as the mean, but the p99.9 and max values (2-3 us, with an occasional
7 us outlier) are dominated by the operating system scheduler, not by this code, and vary
as much between two runs of the *same* variant as between variants. No claim is made
about p99.9 or max.

## What was learned

- The cost per packet was dominated by **reading the clock**, not by locking or copying:
  `Instant::now()` is a `QueryPerformanceCounter` call, about 30 ns here, and the packet
  path made 8 of them (plus the one the polling loop takes before the HID read). Going from
  8 to 5 saved about 90 ns, going from 5 to 2 on most packets saved about 95 ns more.
- The locks were **not** a measurable cost: uncontended, the two `RwLock` writes are cheap,
  and under a 60 Hz reader they hardly ever collide. Replacing them with a lock-free
  structure (for example `arc-swap`, which allocates an `Arc` per packet) was therefore not
  attempted: it would likely cost more than it saves. `ui_stress` shows what contention
  costs in the worst case (about +95 ns; probably cache-line traffic between the two cores rather than blocking, which was not verified).
- `TabletData` is cloned once per packet (into the shared state), not twice: the second
  consumer, the UI channel, takes it by move.

## Perspective

At 1000 Hz a packet arrives every 1 000 000 ns. The path went from about 0.03 % to about
0.01 % of that budget. The change is measurable and keeps the engine thread free for the
parts that matter more (the HID read and the OS injection, neither of which is measured
here), but it is not expected to change how the pen feels, and no end-to-end latency measurement
was made to check.

## Behaviour changes in optB

- Parse / inject timings are sampled (about 60 times per second) instead of measured for
  every packet. They only ever fed the statistics, which were already refreshed at that
  rate, so the statistics are unchanged.
- The `PerfSpike` "packet took more than 5 ms" warning is still raised for every packet,
  from the total time, but it now only includes the parse / inject split when that packet
  happened to be a sampled one.
- `TabletData::parser_time` is now zero on non-sampled packets. Nothing reads it
  (`grep parser_time`): it was dead data.
- Not changed: `Projector::project_relative` still calls `Instant::now()` once per packet
  in relative mode; it could take the packet timestamp as a parameter (about -30 ns in
  that mode).
