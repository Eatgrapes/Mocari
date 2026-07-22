# mocari-ffi

Minimal C ABI adapter for Mocari. The adapter owns the loaded model in Rust and
exposes parameter control plus read-only drawable mesh views to C-compatible
callers such as C#, C++, Unity, Python, Go, and Java JNI. Decoded RGBA8 texture
views are available through `mocari_model_get_texture`.

Mesh pointers returned by `mocari_model_get_mesh` are owned by Rust and remain
valid until the next `mocari_model_update` or `mocari_model_destroy` call.
Callers must not free them. Copy the data when it must outlive that interval.

## Handle contract

`mocari_model_create` returns one owned handle. The caller must pass that handle
to `mocari_model_destroy` exactly once. After destruction, the handle value and
all mesh and texture pointers obtained from it are invalid and must never be
used again. Calling any model function with a destroyed handle, or destroying
the same non-null handle twice, is undefined behavior. Set the host-side handle
to null immediately after destroying it to prevent accidental reuse.

The last error message is thread-local and remains valid until the next FFI
call on the same thread.

A model handle is not thread-safe. Use one handle from one thread, or add
synchronization in the host application.

Build from this directory with `cargo build --release` and use
`include/mocari.h` with the generated dynamic library.

The crate remains platform-neutral. The same source builds as
`mocari_ffi.dll` on Windows, `libmocari_ffi.so` on Linux, and
`libmocari_ffi.dylib` on macOS. The host must provide the platform C runtime and
load the matching artifact. The FFI layer does not enable or depend on the
optional `wgpu` feature.

Run the Rust tests with `cargo test`. On Windows, CI also compiles
`tests/c_smoke.c` with MSVC and links it against the generated DLL to verify the
public C ABI.
