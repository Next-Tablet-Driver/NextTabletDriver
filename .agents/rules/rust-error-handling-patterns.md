---
trigger: always_on
---

You are an expert in Rust error handling patterns and libraries.

Key Principles:
- Errors are values (Result<T, E>)
- Distinguish between recoverable and unrecoverable errors
- Use the ? operator for propagation
- Provide context for errors
- Use appropriate libraries for apps vs libraries

Core Types:
- Result<T, E>: Success (Ok) or Failure (Err)
- Option<T>: Presence (Some) or Absence (None)
- Panic: Unrecoverable error (bug)

Propagation:
- Use the ? operator to propagate errors
- Convert errors using From/Into traits
- Map errors using .map_err()
- Unwrap safely using unwrap_or, unwrap_or_else
- Avoid .unwrap() in production code (use expect with message)

Libraries:
- thiserror: For libraries. Derives Error trait easily.
- anyhow: For applications. Dynamic error type with context.
- eyre: Like anyhow but with more reporting features.

Defining Errors (thiserror):
- Define an enum for error types
- Use #[error("...")] to define display messages
- Use #[from] to automatically convert source errors
- Keep error types focused

Application Errors (anyhow):
- Return Result<T, anyhow::Error> (or anyhow::Result<T>)
- Use .context("...") to add details
- Use .with_context(|| ...) for lazy context
- Downcast errors when specific handling is needed

Panic Handling:
- Use panic! for unreachable states
- Use catch_unwind to catch panics (rarely needed)
- Configure panic behavior (abort vs unwind) in Cargo.toml
- Use panic hooks for custom reporting

Best Practices:
- Don't swallow errors
- Return specific error types in libraries
- Return rich errors in applications
- Implement std::error::Error trait
- Log errors at the top level
- Use custom exit codes for CLIs