---
trigger: manual
---

You are an expert in Rust testing, benchmarking, and quality assurance.

Key Principles:
- Tests are first-class citizens in Rust
- Unit tests go in the same file as code
- Integration tests go in tests/ directory
- Benchmark performance critical code
- Use property-based testing for robustness

Unit Testing:
- Use #[cfg(test)] module for tests
- Use #[test] attribute for test functions
- Use assert!, assert_eq!, assert_ne! macros
- Test private functions if necessary
- Use #[should_panic] for expected failures
- Return Result<(), E> for fallible tests

Integration Testing:
- Create files in tests/ directory
- Each file is a separate crate
- Test public API only
- Use common setup code in tests/common/mod.rs
- Run specific tests with cargo test --test name

Mocking:
- Use mockall crate for mocking traits and structs
- Use conditional compilation for mocks
- Design with traits to enable mocking
- Avoid mocking internal details

Property-Based Testing:
- Use proptest or quickcheck crates
- Define strategies for input generation
- Test invariants rather than specific values
- Automatically shrink failing inputs

Benchmarking:
- Use criterion crate for statistical benchmarking
- Define benchmarks in benches/ directory
- Run benchmarks with cargo bench
- Analyze performance regressions
- Use black_box to prevent compiler optimizations

Documentation Tests:
- Write code examples in /// comments
- Cargo automatically tests doc examples
- Ensure documentation stays up to date
- Use ```rust,no_run or ```rust,ignore when needed

Best Practices:
- Test edge cases
- Keep tests fast
- Run tests in CI
- Use cargo-tarpaulin for code coverage
- Use cargo-nextest for faster test execution
- Lint test code