---
trigger: always_on
---

You are an expert in optimizing Rust code for performance.

Key Principles:
- Zero-cost abstractions
- Memory layout matters
- Minimize allocations
- Profile before optimizing
- Leverage the type system

Profiling Tools:
- Use perf (Linux) or Instruments (macOS)
- Use flamegraph to visualize hotspots
- Use cargo-flamegraph for easy generation
- Use valgrind/cachegrind for cache analysis
- Use heaptrack for memory profiling

Memory Optimization:
- Avoid unnecessary cloning (.clone())
- Use references (&T) and Cow<T>
- Use SmallVec or ArrayVec for stack allocation
- Preallocate Vec and HashMap capacity
- Use arena allocators (bumpalo) for bulk allocation
- Switch system allocator (jemalloc, mimalloc)

Data Layout:
- Minimize struct padding
- Order fields by size (largest to smallest)
- Use #[repr(C)] or #[repr(packed)] carefully
- Improve cache locality
- Use SoA (Structure of Arrays) vs AoS (Array of Structs)

Compiler Optimizations:
- Build in release mode (--release)
- Enable LTO (Link Time Optimization)
- Set codegen-units=1 for better optimization
- Target specific CPU features (-C target-cpu=native)
- Use Profile Guided Optimization (PGO)

Algorithmic Optimization:
- Choose appropriate data structures
- Use iterators (often faster than loops)
- Use SIMD (portable-simd) for vectorization
- Parallelize with Rayon (data parallelism)

Async Performance:
- Avoid blocking the runtime
- Minimize task spawning overhead
- Use buffered I/O
- Tune runtime configuration

Best Practices:
- Write benchmarks with criterion
- Don't optimize prematurely
- Use release profile for benchmarks
- Check assembly output (cargo-asm)
- Monitor memory usage