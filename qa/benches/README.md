# NextTabletDriver - Benchmarking & Performance Engineering

> **Purpose:** define the performance engineering standard for NextTabletDriver.
>
> This directory contains benchmarks used to measure, protect, and continuously improve the driver's performance-critical paths.
>
> The objective is not to make the driver "fast" in isolation. The objective is to make it **predictable, low-latency, allocation-efficient, numerically stable, scalable, and resistant to performance regressions** while preserving correctness.

---

## 1. Benchmarking Philosophy

A tablet driver operates on a continuous stream of input events.

Every additional operation performed for one input sample is potentially multiplied by the number of samples processed during the lifetime of the application.

For that reason, performance must be considered in terms of the complete input path:

```text
Tablet hardware
      │
      ▼
Driver input
      │
      ▼
Pipeline::process()
      │
      ├── hardware / physical specifications
      ├── coordinate transformation
      ├── normalization
      ├── pressure processing
      └── screen projection
      │
      ▼
Output coordinates
      │
      ▼
Operating system / cursor
```

The most important benchmark is therefore not necessarily the smallest mathematical function.

The most important benchmark is the one that answers:

> **How much work does NextTabletDriver perform for one real input sample?**

At the same time, microbenchmarks are required because they allow the source of a regression to be isolated.

The benchmarking hierarchy is therefore:

```text
Level 1 — End-to-end
    Pipeline::process()

Level 2 — Pipeline stages
    Transformer
    Normalizer
    FilterPipeline
    Pressure processing
    Projector

Level 3 — Mathematical primitives
    coordinate transformations
    rotation
    normalization
    projection
    relative movement

Level 4 — Individual filters
    Kalman
    AntiChatter
    statistics
    future filters
```

Each level exists for a different reason.

---

# 2. What "Excellent" Means

A high-quality driver must satisfy several performance properties simultaneously.

## 2.1 Low latency

Each input event must be processed quickly enough that processing latency does not become perceptible.

**Why:** a tablet driver is an interactive real-time system. Average throughput alone is insufficient if individual samples occasionally experience excessive latency.

---

## 2.2 Low variance

Performance must remain stable rather than merely being fast on average.

**Why:** input latency spikes are more damaging to interaction quality than a small increase in average processing time.

The benchmark suite should therefore pay attention to:

* median / typical execution time;
* high-percentile behavior when available;
* variance;
* outliers;
* changes in distribution.

A benchmark that becomes slightly faster on average but develops large latency spikes should not automatically be considered an improvement.

---

## 2.3 No unnecessary allocations

The hot path should avoid heap allocation whenever practical.

**Why:** `Pipeline::process()` can execute once for every input sample. A seemingly insignificant allocation can become a substantial source of allocator pressure at high polling rates.

Pay particular attention to:

* `Vec` growth;
* temporary collections;
* `String` creation;
* boxed objects;
* cloning;
* heap-backed intermediate structures;
* dynamic dispatch introduced solely for convenience.

---

## 2.4 Predictable complexity

Performance should scale predictably with configuration complexity.

**Why:** adding one filter should not unexpectedly produce super-linear behavior or introduce unrelated overhead into the rest of the pipeline.

The benchmark suite must therefore test:

```text
0 filters
1 filter
2 filters
N filters
```

where relevant.

---

## 2.5 Numerical stability

Mathematical optimization must never silently change the driver's behavior.

**Why:** coordinate processing is fundamentally numerical. An optimization that reduces execution time while introducing accumulated floating-point error is not a valid optimization.

Performance and numerical correctness must therefore be evaluated together.

---

# 3. Primary Benchmark: `Pipeline::process()`

**Target:**

```text
src/engine/pipeline/mod.rs
```

The primary benchmark must measure:

```rust
Pipeline::process(...)
```

This is the principal performance KPI for the driver.

## Why?

`Pipeline::process()` represents the real processing path used by the driver.

It combines:

1. input validation;
2. device specification access;
3. physical coordinate transformation;
4. normalization;
5. filtering;
6. pressure processing;
7. pressure curve evaluation;
8. absolute or relative projection;
9. output generation.

Optimizing a mathematical function without measuring this path can produce misleading conclusions.

For example:

```text
10% faster coordinate conversion
```

does not necessarily mean:

```text
10% faster driver
```

if coordinate conversion represents only a small fraction of total processing time.

The end-to-end pipeline benchmark prevents this mistake.

---

# 4. Pipeline Scenarios

`Pipeline::process()` must not be benchmarked under only one configuration.

At minimum, benchmark the following scenarios.

## 4.1 Absolute mode

**Why:** absolute positioning is a fundamental operating mode and provides a clean baseline for the pipeline.

Measure:

```text
raw input
→ transformation
→ normalization
→ pressure
→ absolute projection
```

---

## 4.2 Relative mode

**Why:** relative projection has state and timing behavior that absolute projection does not have.

In particular, `Projector::project_relative()` involves:

* previous state;
* relative deltas;
* reset behavior;
* timing through `Instant::now()`.

This path must therefore be measured separately.

Do not use relative projection as a substitute for benchmarking the pure mathematical function.

---

## 4.6 Pressure disabled

**Why:** optional features should not impose unnecessary computational cost when disabled.

Compare:

```text
pressure disabled
pressure enabled
```

The difference represents the actual cost of pressure processing.

---

## 4.7 Pressure curves

**Why:** pressure curve evaluation is part of the per-sample hot path and may become significant when more complex curves are introduced.

Benchmark each supported curve family where practical.

The objective is not to optimize a curve independently of the driver, but to determine whether curve complexity materially affects total processing cost.

---

# 5. Transformer

**Target:**

```text
src/engine/pipeline/transformer.rs
```

Primary function:

```rust
Transformer::execute()
```

## Why?

The transformer converts raw tablet coordinates into physical coordinates.

It is both:

* numerically important;
* executed for every processed input sample.

The transformer also contains caching behavior for hardware and physical dimensions.

That caching behavior must be explicitly tested.

---

## 5.1 Warm-cache benchmark

Measure repeated execution after the transformer has already calculated its multipliers.

**Why:** this represents normal steady-state driver operation.

The benchmark must not accidentally measure initialization on every iteration.

---

## 5.2 Cache initialization

Measure the cost of the first calculation separately.

**Why:** initialization cost and steady-state cost answer different questions.

A fast steady state does not mean that expensive repeated initialization is acceptable.

---

## 5.3 Cache invalidation

Benchmark the behavior when hardware or physical dimensions change.

**Why:** cache invalidation is necessary for correctness, but it must not accidentally cause repeated recalculation during normal operation.

---

## 5.4 Epsilon-sensitive dimensions

The transformer currently contains tolerance-sensitive cache behavior.

Benchmark representative values around the tolerance boundary.

**Why:** floating-point comparisons near cache thresholds can create pathological behavior where tiny changes repeatedly invalidate cached values.

The goal is to ensure that the optimization remains both:

* numerically correct;
* operationally stable.

---

# 6. Normalizer

**Target:**

```text
src/engine/pipeline/normalizer.rs
```

Primary operation:

```rust
Normalizer::execute()
```

which delegates to the coordinate normalization mathematics.

## Why?

Normalization is a hot-path transformation and should remain close to the cost of the underlying mathematical operation.

The benchmark should verify that the abstraction layer does not introduce unnecessary overhead.

---

# 7. Core Mathematical Transformations

**Target:**

```text
src/core/math/transform.rs
```

These functions should have dedicated microbenchmarks.

---

## 7.1 `rotate_point`

Benchmark representative rotation angles:

```text
0°
45°
90°
180°
arbitrary angle
```

## Why?

Rotation contains trigonometric operations and is potentially more expensive than simple arithmetic.

The benchmark establishes whether repeated calculations can be reduced or cached without changing semantics.

---

## 7.2 `physical_to_normalized`

Benchmark:

```text
center point
corner points
arbitrary points
different active areas
```

## Why?

This function converts physical coordinates into normalized coordinates used by later pipeline stages.

It is mathematically fundamental and therefore an ideal target for isolated optimization.

---

## 7.3 `normalized_to_screen`

Benchmark:

```text
center
corners
near-boundary coordinates
outside-range coordinates
```

## Why?

Screen projection contains clamping behavior.

Boundary handling must remain correct while optimization is performed.

A benchmark must therefore include pathological inputs rather than measuring only ideal values.

---

## 7.4 `apply_relative_delta`

Benchmark representative:

```text
small movement
large movement
rotated movement
zero movement
```

## Why?

This is the pure mathematical core of relative positioning.

It should be benchmarked independently of `Instant::now()` and other state-management overhead.

This makes it possible to distinguish:

```text
mathematical cost
```

from:

```text
state/timing cost
```

---

# 8. Projector

**Target:**

```text
src/engine/pipeline/projector.rs
```

The projector must be considered in two distinct categories.

---

## 8.1 `project_absolute()`

Benchmark absolute projection.

**Why:** this is a stateful wrapper around the mathematical screen projection and therefore represents real production behavior.

---

## 8.2 `project_relative()`

Benchmark the complete relative projection path separately.

**Why:** it includes timing and state management that cannot be represented by a pure mathematical benchmark.

Do not interpret its benchmark as a measurement of `apply_relative_delta()` alone.

---

## 8.3 Reset behavior

Benchmark both:

```text
continuous movement
inactivity/reset path
```

**Why:** reset logic can introduce conditional branches and timing operations that are invisible in a normal mathematical benchmark.

---

# 11. Benchmark the Cost of Abstractions

Performance work must not assume that abstraction is either good or bad.

It must be measured.

Potential areas include:

```text
trait dispatch
Box<dyn Filter>
state wrappers
helper functions
configuration lookups
```

For example:

```rust
Vec<Box<dyn Filter>>
```

may introduce dynamic dispatch.

That is not automatically a problem.

## Why?

Dynamic dispatch has a cost, but replacing it with generics, enums, or static dispatch may increase code complexity or binary size.

The correct engineering question is:

> Is the abstraction's runtime cost significant enough to justify changing the architecture?

Benchmarks must answer that question before architectural optimization is attempted.

---

# 12. Memory and Allocation Behavior

Execution time alone is insufficient.

The benchmark suite should eventually include allocation-oriented measurements for hot-path code.

Pay particular attention to:

```text
Pipeline::process()
FilterPipeline::process()
individual filters
statistics collection
temporary data structures
```

## Why?

A function can have an acceptable execution time while allocating repeatedly.

At high event rates, repeated allocation can cause:

* allocator overhead;
* cache disruption;
* memory traffic;
* increased variance;
* eventual pressure on the system allocator.

The desired steady-state property is:

> **No unnecessary heap allocation per input sample.**

---

# 13. Cache Behavior

The benchmark suite must distinguish:

```text
cold state
warm state
```

where applicable.

## Why?

Modern CPUs are heavily dependent on cache locality.

A benchmark that initializes all state inside the measured operation may produce numbers that do not represent real driver execution.

Examples include:

* transformer multiplier caches;
* filter state;
* projector state;
* reusable buffers.

Steady-state benchmarks should reproduce steady-state driver behavior.

---

# 14. Benchmark Setup Must Not Pollute Measurements

Benchmark setup must be outside the measured operation whenever the setup is not part of the production hot path.

For example:

```text
Configuration creation
Pipeline construction
Filter construction
Initial cache population
```

should generally be performed before measurement.

## Why?

Otherwise the benchmark measures:

```text
setup + processing
```

instead of:

```text
processing
```

and the result becomes difficult to interpret.

When state must be recreated between iterations, use Criterion's batched iteration mechanisms rather than manually mixing setup and measurement.

---

# 15. Avoid Compiler-Eliminated Work

Benchmark inputs and outputs must be consumed using appropriate black-box mechanisms.

Prefer the modern Rust standard-library primitive:

```rust
std::hint::black_box
```

where appropriate.

## Why?

A compiler is allowed to eliminate calculations whose results are provably unused.

Without black-box protection, a benchmark may measure optimized-away code instead of the actual operation.

A benchmark is only useful if it measures the work that production code actually performs.

---

# 16. Benchmark Input Design

Benchmarks must use realistic data.

Do not benchmark only:

```text
x = 0
y = 0
pressure = 0
```

unless zero values are specifically the scenario being tested.

Use representative distributions including:

```text
center
edges
corners
small movements
large movements
random realistic coordinates
different pressure values
different rotations
```

## Why?

Optimizers, branch predictors, caches, and mathematical code can behave differently depending on input distributions.

A benchmark using one artificial input can produce an unrealistically favorable result.

---

# 17. Prevent Benchmark Gaming

A benchmark must never be optimized merely to produce a better number.

Every optimization must answer three questions:

### 1. What became faster?

### 2. Why did it become faster?

### 3. Did the observable behavior remain identical?

## Why?

A smaller benchmark number has no value if the implementation:

* loses precision;
* changes cursor behavior;
* breaks edge cases;
* changes filter semantics;
* increases latency variance;
* introduces hidden allocations;
* sacrifices maintainability.

Performance engineering is optimization under correctness constraints.

---

# 18. Correctness Is a Performance Constraint

Every performance optimization must preserve the existing mathematical and behavioral contracts.

Relevant tests include:

```text
coordinate conversion
normalization
rotation
screen clamping
relative movement
pressure threshold
pressure curves
filter behavior
cache behavior
reset behavior
```

## Why?

A driver cannot trade correctness for throughput.

For this project, an optimization is successful only when:

```text
correctness preserved
+
performance improved
+
complexity justified
```

All three conditions matter.

---

# 19. Regression Detection

Benchmarks exist primarily to prevent regressions.

A benchmark should therefore be compared against historical results rather than interpreted as an isolated number.

Monitor changes in:

```text
median execution time
throughput
variance
allocation behavior
individual stage cost
end-to-end pipeline cost
```

## Why?

A driver can slowly become slower through many individually insignificant changes.

Without historical measurements, this degradation is difficult to detect.

---

# 20. End-to-End Performance Must Remain the Final Authority

Microbenchmarks are diagnostic tools.

The final performance question remains:

```text
How expensive is one complete input sample?
```

Therefore:

```text
Pipeline::process()
```

must remain the principal performance benchmark.

## Why?

Optimizing individual functions without observing the complete pipeline can lead to local improvements with no meaningful system-level benefit.

The end-to-end benchmark prevents local optimization from becoming the project's primary objective.

---

# 21. Performance Budget

The project should eventually define explicit performance budgets.

For example:

```text
Pipeline::process()
    target: defined after baseline measurements

Transformer
    target: defined after baseline measurements

Normalizer
    target: defined after baseline measurements

FilterPipeline
    target: defined per configuration

Individual filters
    target: defined from measured contribution
```

Do not invent arbitrary limits before collecting reliable baseline measurements.

## Why?

A performance budget must represent actual hardware and workload requirements.

An arbitrary threshold can either:

* reject perfectly acceptable implementations;
* or allow unacceptable regressions.

The correct process is:

```text
Measure baseline
      ↓
Identify realistic workload
      ↓
Determine acceptable budget
      ↓
Enforce regression policy
```

---

# 22. Benchmark Matrix

The benchmark suite should evolve toward a matrix similar to:

| Area                  | Baseline        | Variants            | Why                            |
| --------------------- | --------------- | ------------------- | ------------------------------ |
| `Pipeline::process()` | no filters      | absolute / relative | Measures real driver cost      |
| Transformer           | warm cache      | cold / invalidated  | Detects cache regressions      |
| Normalizer            | normal input    | boundaries          | Protects numerical path        |
| Rotation              | arbitrary point | multiple angles     | Measures trig-heavy operation  |
| Normalization         | normal input    | boundaries          | Protects coordinate mapping    |
| Projection            | normal input    | boundary values     | Protects clamping              |
| Relative delta        | small movement  | large / rotated     | Measures pure relative math    |
| Pressure              | disabled        | enabled             | Measures optional feature cost |
| Pressure curves       | simple          | complex             | Measures curve overhead        |
| Statistics            | disabled        | enabled             | Measures instrumentation cost  |

---

# 23. What Must Be Watched During Optimization

The following files are performance-sensitive and should receive special scrutiny during optimization work.

## `src/engine/pipeline/mod.rs`

**Watch:**

```text
Pipeline::process()
```

**Why:** this is the complete per-input hot path and therefore the highest-value performance target.

---

## `src/engine/pipeline/transformer.rs`

**Watch:**

```text
Transformer::execute()
cache invalidation
multiplier calculation
```

**Why:** coordinate transformation is performed continuously, while cache behavior determines whether expensive calculations are repeated unnecessarily.

---

## `src/engine/pipeline/normalizer.rs`

**Watch:**

```text
Normalizer::execute()
```

**Why:** normalization is a per-sample mathematical operation and should remain close to the minimum possible abstraction overhead.

---

## `src/engine/pipeline/projector.rs`

**Watch:**

```text
project_absolute()
project_relative()
reset logic
state updates
```

**Why:** projection is the final coordinate stage and relative mode introduces state and timing behavior that can affect latency.

---

## `src/core/math/transform.rs`

**Watch:**

```text
rotate_point()
physical_to_normalized()
normalized_to_screen()
apply_relative_delta()
```

**Why:** these are small, frequently executed mathematical primitives where low-level optimization can have measurable effects.

---

## `src/filters/mod.rs`

**Watch:**

```text
FilterPipeline::process()
```

**Why:** filter-chain overhead grows with the number of active filters and can become a dominant component of processing cost.

---

## Individual filter implementations

**Watch:**

```text
Kalman
AntiChatter
Stats
StatsServer
```

**Why:** stateful filters can have substantially different computational characteristics, and one expensive filter can dominate the entire chain.

---

# 24. What Should Not Be Optimized Prematurely

Not every line of code deserves a benchmark.

Avoid optimizing code solely because it looks inefficient.

Examples:

```text
rare initialization paths
configuration parsing
one-time setup
administrative commands
non-critical diagnostics
```

unless profiling demonstrates that they matter.

## Why?

Optimization has a cost.

It can increase:

* complexity;
* maintenance burden;
* review difficulty;
* bug surface;
* architectural coupling.

The project should optimize measured bottlenecks rather than visual impressions.

---

# 25. Benchmarking Rules

Every benchmark added to this directory should follow these rules.

### Rule 1 — Measure production behavior

**Why:** synthetic benchmarks are useful only when they represent a meaningful production workload.

### Rule 2 — Keep setup outside the timed region

**Why:** setup cost must not contaminate steady-state measurements.

### Rule 3 — Use realistic inputs

**Why:** unrealistic inputs can produce misleading branch, cache, and optimizer behavior.

### Rule 4 — Protect against dead-code elimination

**Why:** otherwise the benchmark may not measure the intended computation.

### Rule 5 — Benchmark stateful components correctly

**Why:** state mutation can otherwise make repeated iterations incomparable.

### Rule 6 — Separate cold-start and steady-state measurements

**Why:** they represent different operational conditions.

### Rule 7 — Benchmark both micro and end-to-end behavior

**Why:** microbenchmarks identify causes; end-to-end benchmarks measure actual impact.

### Rule 8 — Never sacrifice correctness for a benchmark number

**Why:** a faster incorrect driver is a regression.

### Rule 9 — Investigate unexpected improvements

**Why:** a dramatic improvement can indicate a real optimization, a changed workload, or an invalid benchmark.

### Rule 10 — Investigate unexpected regressions

**Why:** small regressions in hot code can become significant at high input rates.

---

# 26. Interpreting Results

A benchmark result must never be interpreted as:

> "Lower is always better."

The correct interpretation is:

```text
Is the measured workload representative?
        ↓
Is the result statistically meaningful?
        ↓
Did the implementation change?
        ↓
Did correctness remain unchanged?
        ↓
Did the end-to-end pipeline improve?
        ↓
Did variance or allocations regress?
```

Only then should an optimization be considered successful.

---

# 27. Criterion

Criterion is the expected benchmarking framework for this directory.

Run the complete benchmark suite with:

```bash
cargo bench
```

Individual benchmark targets may be executed as needed.

## Why Criterion?

The project requires more than a single execution time.

Statistical benchmarking is necessary to distinguish:

```text
real performance changes
```

from:

```text
normal measurement noise
```

Criterion also provides a consistent framework for comparing successive implementations.

---

# 28. Benchmark Organization

The intended structure is:

```text
benches/
├── README.md
├── pipeline.rs
├── transforms.rs
└── filters.rs
```

Additional benchmark files may be introduced when a new performance domain becomes sufficiently important.

## `pipeline.rs`

Contains end-to-end and pipeline-stage benchmarks.

**Why:** the pipeline represents the real driver workload.

---

## `transforms.rs`

Contains pure mathematical benchmarks.

**Why:** mathematical primitives require isolated measurements to identify optimization opportunities.

---

## `filters.rs`

Contains filter-chain and individual-filter benchmarks.

**Why:** filtering is stateful and configuration-dependent, so it requires dedicated analysis.

---

# 29. Benchmark Naming

Benchmark names must describe the workload rather than the implementation detail.

Prefer:

```text
pipeline/absolute/no_filters
pipeline/relative/no_filters
transform/physical_to_normalized
transform/normalized_to_screen
```

Avoid vague names such as:

```text
test1
fast_path
new_impl
optimized
benchmark
```

## Why?

Benchmark names become historical identifiers.

They must remain meaningful after the implementation changes.

---

# 30. Optimization Workflow

Every serious optimization should follow this process:

```text
1. Establish a baseline
        ↓
2. Identify the bottleneck
        ↓
3. Explain why it is expensive
        ↓
4. Implement the smallest justified change
        ↓
5. Run correctness tests
        ↓
6. Run microbenchmarks
        ↓
7. Run end-to-end benchmarks
        ↓
8. Compare distributions
        ↓
9. Inspect generated behavior when necessary
        ↓
10. Keep the change only if the system-level result justifies it
```

## Why?

This prevents optimization from becoming guesswork.

The benchmark is evidence, not the objective itself.

---

# 31. Advanced Investigation

When a benchmark identifies a significant bottleneck, deeper analysis may be required.

Potential tools include:

```text
cargo bench
Criterion reports
cargo test
cargo clippy
cargo llvm-lines
cargo bloat
perf
flamegraph
operating-system profilers
CPU performance counters
```

Use the appropriate tool for the question.

For example:

```text
Criterion
→ "Did execution time change?"

Flamegraph
→ "Where is execution time spent?"

cargo llvm-lines
→ "Which code contributes to generated LLVM?"

cargo bloat
→ "What contributes to binary size?"

perf / hardware counters
→ "What is the CPU actually doing?"
```

## Why?

No single benchmark can explain every performance problem.

---

# 32. Release-Profile Validation

Benchmarks must be interpreted in the context of the project's optimized build configuration.

The release configuration currently emphasizes:

```text
opt-level = 3
LTO
codegen-units = 1
incremental = false
```

## Why?

Optimization-sensitive code can behave very differently between debug and optimized builds.

Performance conclusions must therefore be based on the configuration representative of actual driver execution.

---

# 33. Performance and Architecture

Performance optimization must not be isolated from architecture.

A perfect driver should have:

```text
clear ownership
predictable data flow
minimal unnecessary work
well-defined state
small hot-path abstractions
controlled allocations
stable numerical behavior
measurable performance
```

## Why?

A fast implementation that cannot be understood or safely modified is not an exemplary engineering result.

The best architecture is one where performance characteristics are understandable from the code structure.

---

# 34. Long-Term Performance Goals

The benchmark suite should progressively answer the following questions.

### Input processing

> How much CPU time does one tablet sample require?

### Scaling

> How does processing cost change as filters are added?

### Mathematical cost

> Which coordinate transformations dominate execution time?

### State management

> Which state updates contribute measurable overhead?

### Memory

> Does the steady-state hot path allocate?

### Stability

> Does performance remain stable under continuous input?

### Optional features

> What is the exact cost of enabling each optional feature?

### Regression resistance

> Can a future change make the driver slower without being detected?

### Architecture

> Is a measured bottleneck caused by an algorithm, data structure, abstraction, allocation, synchronization, or external operation?

These questions define the purpose of this directory.

---

# 35. Definition of an Exemplary State

NextTabletDriver should be considered performance-exemplary only when the following properties are demonstrated rather than assumed:

* `Pipeline::process()` has a measured and documented baseline.
* Major pipeline stages have isolated benchmarks.
* Mathematical hot spots have dedicated microbenchmarks.
* Filter costs are measurable individually and as a chain.
* Cold-start and steady-state behavior are distinguished.
* Cache behavior is explicitly tested.
* Realistic workloads are represented.
* Hot-path allocations are understood and minimized.
* Numerical behavior remains correct under optimization.
* Performance regressions can be detected automatically.
* Benchmark results are reproducible.
* Major optimizations are justified by measurements.
* End-to-end performance remains the final validation criterion.
* Performance improvements do not create disproportionate architectural complexity.

---

# 36. Final Principle

The objective of this directory is not:

> **"Make every function as fast as possible."**

The objective is:

> **"Make the complete driver perform the minimum necessary work, with predictable latency and stable numerical behavior, while keeping the architecture understandable and maintainable."**

Every benchmark should exist because it answers a concrete engineering question.
Every optimization should exist because measurements demonstrate that it solves a real problem.
Every regression should be detectable before it becomes a user-visible problem.
And every performance claim should be reproducible from evidence.
That is the standard this directory is intended to enforce.
