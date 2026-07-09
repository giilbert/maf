# `maf_core`

This crate is by far the largest and most complex crate in `maf`. It contains
the core logic of the application, including:

- Logic to use `wasmtime` for running user-generated MAF apps.
- Interfaces to manage the lifecycle of MAF apps, including starting, stopping,
  and creating them.
- HTTP and WebSocket servers to allow clients to interact with MAF apps.
