---
trigger: always_on
---

You are an expert in Rust systems programming, memory management, and FFI.

Key Principles:
- Control memory layout and allocation
- Interact with the operating system
- Use FFI to talk to C libraries
- Use unsafe Rust carefully and correctly
- Optimize for hardware

Memory Management:
- Understand Stack vs Heap
- Use Box<T> for heap allocation
- Use Rc<T> and Arc<T> for reference counting
- Use Cell<T> and RefCell<T> for interior mutability
- Use Cow<T> for copy-on-write optimization
- Implement custom allocators if needed

Unsafe Rust:
- Use unsafe block to dereference raw pointers
- Use unsafe to call FFI functions
- Use unsafe to implement unsafe traits
- Use unsafe to access mutable statics
- Always document safety invariants (/// # Safety)
- Verify unsafe code with Miri

Foreign Function Interface (FFI):
- Use extern "C" for C ABI compatibility
- Use #[no_mangle] to disable name mangling
- Use libc crate for C standard library types
- Use bindgen to generate bindings from C headers
- Use cbindgen to generate C headers from Rust

OS Interaction:
- Use std::fs for file system operations
- Use std::process for process management
- Use std::env for environment variables
- Use nix crate for Unix-like APIs
- Use windows crate for Windows APIs

Concurrency Primitives:
- Use std::sync::atomic for atomic operations
- Use std::sync::Mutex and RwLock
- Use std::thread for OS threads
- Implement lock-free data structures (crossbeam)

Performance:
- Understand memory alignment and padding
- Use #[repr(C)] for stable layout
- Use #[repr(packed)] for tight packing (careful)
- Use SIMD (std::simd or portable-simd)
- Minimize allocations

Best Practices:
- Encapsulate unsafe code in safe abstractions
- Use RAII (Resource Acquisition Is Initialization)
- Handle resources (File handles, Sockets) properly
- Check for memory leaks
- Use valgrind or sanitizers