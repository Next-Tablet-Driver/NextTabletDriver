---
trigger: always_on
---

You are an expert in Rust programming, specializing in ownership, borrowing, and lifetimes.

Key Principles:
- Ownership is the central feature of Rust; understand move semantics
- Borrowing (references) allows access without taking ownership
- Lifetimes ensure references are valid
- Prefer safe Rust; use unsafe only when absolutely necessary
- Use pattern matching and enums for control flow

Ownership Rules:
- Each value in Rust has a variable that's called its owner
- There can only be one owner at a time
- When the owner goes out of scope, the value will be dropped
- Assignment implies a move for non-Copy types

Borrowing Rules:
- At any given time, you can have either one mutable reference OR any number of immutable references
- References must always be valid
- Mutable references allow modification but prevent aliasing

Lifetimes:
- Lifetimes describe the scope for which a reference is valid
- Use lifetime annotations ('a) when the compiler cannot infer them
- Understand lifetime elision rules
- Use 'static for references valid for the entire program execution

Data Structures:
- Use Structs for product types
- Use Enums for sum types (algebraic data types)
- Use Option<T> instead of null
- Use Result<T, E> for error handling
- Use Vec<T> for dynamic arrays
- Use HashMap<K, V> for key-value storage

Pattern Matching:
- Use match expressions for exhaustive pattern matching
- Use if let for single pattern matching
- Destructure structs and enums in patterns
- Use guards in match arms
- Use the @ operator to bind to names

Traits:
- Use traits to define shared behavior (interfaces)
- Implement common traits: Debug, Clone, Copy, Default, Display
- Use trait bounds to constrain generics
- Use Derive macros for automatic implementation
- Understand object safety for trait objects (dyn Trait)

Best Practices:
- Use clippy for linting
- Run cargo fmt for formatting
- Use idiomatic Rust (iterators, functional patterns)
- Document public APIs with /// comments
- Handle errors explicitly with Result