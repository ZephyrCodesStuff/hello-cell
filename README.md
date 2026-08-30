# hello-cell: Bare-Metal Rust on PlayStation 3 (Cell B.E.)

A modern `#![no_std]` **Rust** runtime and application environment executing directly on the **PlayStation 3's Cell Broadband Engine (PPE)** under the GameOS LV2 kernel.

Includes dynamic heap allocation (`alloc`), high-performance native ELF linking via **`mold`**, declarative SPRX dynamic linking (`sprx.toml`), and a native **HTTP server** over PS3 sockets.

---

## Quickstart: Build & Run in 60 Seconds

### Requirements
1. **Rust Nightly**:
   ```powershell
   rustup toolchain install nightly
   rustup component add rust-src --toolchain nightly
   ```
2. **`mold` Linker**: Installed and available in your `PATH` ([mold releases](https://github.com/rui314/mold/releases)).
3. **`make_fself.exe`**: Available in your `PATH` (from the PS3 SDK tools).

### Building Your First Binary
Run the automated build script:
```powershell
.\build.ps1
```

This generates `EBOOT.ELF` and `EBOOT.BIN` in the root folder in ~1.5 seconds.

### Running & Testing
- **RPCS3**: Drag and drop `EBOOT.BIN` or `EBOOT.ELF` directly onto RPCS3.
- **PS3 Hardware (CFW / HEN / DEX)**: Deploy `EBOOT.BIN` to `/dev_hdd0/game/HELLOCELL/USRDIR/` or launch over network via OpenTM / ProDG.
- **Test the HTTP Server**:
  While running on hardware or RPCS3, open your browser or terminal and run:
  ```powershell
  curl http://<PS3_IP>:8080
  ```
  Response:
  ```http
  HTTP/1.1 200 OK
  Content-Type: text/plain; charset=utf-8
  Content-Length: 75
  Server: PS3-CellOS-Rust/1.0

  Hello, Cell!

  Stats:
  - Active Allocs: 4
  - Active Bytes: 136 bytes
  - Claimed Total: 4 MB
  ```

---

## Features & Architecture

- **PPC64 ELFv1 ABI**: Custom target definition ([`powerpc-unknown-cellos.json`](powerpc-unknown-cellos.json)) generating 64-bit Big-Endian PowerPC machine code with compliant `.TOC.` (Table of Contents) addressing.
- **Native Direct Linking via `mold`**: Standard `cargo build` links the binary directly using `mold` with no intermediate static archives or legacy linker scripts.
- **Declarative SPRX Dynamic Linking ([`sprx.toml`](sprx.toml))**: Zero manual assembly needed to import PS3 system libraries (`cellSysmodule`, `sys_net`, `cellNetCtl`, etc.). Declare functions and FNIDs in TOML.
- **`moldier` PS3 Toolchain Utility ([`moldier/`](moldier/))**: Built-in Rust tool that auto-generates assembly stubs and applies Sony LV2 kernel headers (`PT_SCE_PROC_PARAM`, `PT_SCE_PROC_PRX_PARAM`, and OPD descriptor packing).
- **Dynamic Memory Allocation (`alloc`)**: O(1) heap allocation powered by **Talc 5.1**, dynamically claiming 4 MB chunks on demand from LV2 `sys_memory_allocate`.
- **Embedded HTTP / TCP Socket Stack ([`src/net.rs`](src/net.rs))**: Native non-blocking and blocking TCP server support over GameOS `sys_net` and `cellNetCtl`.
- **Console TTY Output ([`src/io.rs`](src/io.rs))**: Formatted `print!` and `println!` macros routed through `SYS_TTY_WRITE` (Syscall 403).
- **Type-Safe Kernel Syscall DSL ([`src/syscalls.rs`](src/syscalls.rs))**: Centralized assembly wrapper managing 128-byte stack linkage frames and TOC register (`r2`) preservation.

---

## Project Layout

```
hello-cell/
├── .cargo/
│   └── config.toml          # Native mold linker flags and target configuration
├── moldier/                 # Host tool: SPRX code generator and PS3 ELF patcher
│   ├── Cargo.toml
│   └── src/main.rs
├── src/
│   ├── main.rs              # Application entry point (rust_main) & HTTP server loop
│   ├── entry.rs             # _start_code PowerPC64 ELFv1 bootstrap
│   ├── syscalls.rs          # PS3 LV2 kernel syscall bindings
│   ├── allocator.rs         # Talc 5.1 dynamic memory allocator & Mutex
│   ├── net.rs               # PS3 TCP socket & network lifecycle abstractions
│   ├── io.rs                # print! / println! TTY output
│   ├── mem.rs               # C runtime intrinsics (memcpy, memset, memcmp, memmove)
│   ├── panic.rs             # Custom panic handler reporting to TTY
│   └── sprx.s               # Auto-generated assembly stubs (from sprx.toml)
├── sprx.toml                # Declarative SPRX library & FNID import manifest
├── powerpc-unknown-cellos.json  # PowerPC64 ELFv1 target specification
├── build.ps1                # One-command build & packaging pipeline
└── Cargo.toml               # Cargo package manifest
```

---

## Adding PS3 System Libraries

To call any PlayStation 3 SPRX function:

1. Add the library name and function FNID to [`sprx.toml`](sprx.toml):
   ```toml
   [libraries.cellAudio]
   module_id = 0x0009
   functions = [
       { name = "cellAudioInit", fnid = 0x8C090886 },
       { name = "cellAudioQuit", fnid = 0x164C82FB },
   ]
   ```

2. Declare the `extern "C"` signature in your Rust code:
   ```rust
   extern "C" {
       pub fn cellAudioInit() -> i32;
       pub fn cellAudioQuit() -> i32;
   }
   ```

3. Run `.\build.ps1`. The stubs, trampolines, and PRX headers are generated and bound automatically.

---

## Manual Build Steps

If you prefer running the build commands individually:

```powershell
# 1. Generate SPRX assembly stubs from sprx.toml
cargo run -p moldier -- gen-stubs --config sprx.toml --output src/sprx.s

# 2. Build the ELF binary directly with mold
cargo +nightly build --target powerpc-unknown-cellos.json -Z unstable-options -Z build-std=core,alloc,compiler_builtins -Z json-target-spec -p hello-cell

# 3. Patch ELF with PS3 kernel headers and packed OPD descriptors
cargo run -p moldier -- patch target/powerpc-unknown-cellos/debug/EBOOT.ELF

# 4. Encrypt into signed EBOOT.BIN
make_fself target/powerpc-unknown-cellos/debug/EBOOT.ELF EBOOT.BIN
```

---

## License

AGPL-3.0
