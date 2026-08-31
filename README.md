<h1 align="center">rust-ps3</h1>

<p align="center">
    <a href="https://www.rust-lang.org/">
        <img src="https://img.shields.io/badge/rust-nightly-orange.svg?style=flat-square">
    </a>
    <a href="https://github.com/rui314/mold">
        <img src="https://img.shields.io/badge/linker-mold-blue.svg?style=flat-square">
    </a>
    <a href="LICENSE">
        <img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square">
    </a>
    <a href="#">
        <img src="https://img.shields.io/badge/platform-PlayStation%203%20(CellOS)-purple.svg?style=flat-square">
    </a>
</p>
<p align="center">
    A Rust runtime, SDK, and toolchain for the Sony PlayStation 3 (Cell Broadband Engine) <i>(formerly <code>hello-cell</code>)</i>.
</p>

```rust
#![no_std]
#![no_main]

use ps3::alloc::boxed::Box;
use ps3::alloc::format;
use ps3::allocator;
use ps3::println;
use ps3::sys::syscalls;

#[no_mangle]
pub extern "C" fn rust_main() -> i32 {
    println!("Hello PlayStation 3 from Rust!");

    let boxed_val = Box::new(0x1337_4242_u64);
    println!("Boxed Value: {:#X}", *boxed_val);

    let stats = allocator::get_heap_stats();
    println!("Active Allocs: {}, Active Bytes: {} bytes", stats.active_allocations, stats.active_bytes);

    0
}
```

See `examples/` directory for sample programs (`examples/hello-world` and `examples/http-server`).

## Features / Roadmap

- [x] `core` & `alloc` support (`talc` fast allocator backed by `sys_memory_allocate` / `sys_memory_free`)
- [x] PowerPC64 ELFv1 ABI support (Big-Endian Cell PPU)
- [x] LV2 Syscall dispatcher (direct inline assembly syscalls via `sc 11`)
- [x] TTY / Standard I/O output via `sys_tty_write`
- [x] `moldier`: ELF post-linker and OPD function descriptor patcher for `mold`
- [x] Automated SPRX dynamic stub generator (`sprx.toml`) and linker binding
- [x] Network socket layer (`sys_net` / `cellSysmodule` stubs, TCP listener & streams)
- [x] Direct compilation via Rust `json-target-spec` without legacy cross-compiler toolchains
- [ ] SPU (Synergistic Processing Unit) toolchain & inter-processor communication (MFC DMA / mailboxes)
- [ ] RSX / libgcm 3D graphics hardware acceleration
- [ ] `std` support
- [ ] Automated self / `EBOOT.BIN` packaging

### What about PSL1GHT / Cell SDK?

Traditional PS3 homebrew relies on the legacy GCC-based **PSL1GHT** SDK or the official Sony Cell SDK. 

`rust-ps3` (formerly `hello-cell`) is a **standalone, modern toolchain and runtime** built directly on top of upstream LLVM/rustc:
- Targets 64-bit PowerPC ELFv1 (`powerpc64-unknown-linux-gnu` reference target spec, customized to `powerpc64-sony-ps3` for CellOS and PPU).
- Links binaries using the modern high-performance [`mold`](https://github.com/rui314/mold) linker.
- Patches the final binary with **`moldier`**, a small(ish) post-linker tool that generates SPRX import stubs from TOML definitions and patches Sony LV2 Official Procedure Descriptors (OPD) and TOC relocations directly into the final ELF.

## Dependencies

To compile for the PS3, you will need a Rust **nightly** toolchain with the `rust-src` component:

```sh
$ rustup default nightly && rustup component add rust-src
```

You also need the [`mold`](https://github.com/rui314/mold) linker installed and available in your `PATH`:

```sh
# macOS (Homebrew)
$ brew install mold

# Arch Linux
$ pacman -S mold

# Ubuntu / Debian
$ apt install mold
```

*(Optional)* If you want to automatically generate signed `EBOOT.BIN` files for retail/CFW PS3s, install `make_fself` into your `PATH`.

## Building & Running Examples

To build one of the examples (e.g. `hello-world` or `http-server`), run:

```sh
# Linux / macOS
$ ./scripts/build.sh hello-world
# or
$ ./scripts/build.sh http-server

# Windows (PowerShell)
$ .\scripts\build.ps1 -TargetExample hello-world
# or
$ .\scripts\build.ps1 -TargetExample http-server
```

The build script will:
1. Compile the host tool `moldier`.
2. Generate assembly stubs (`ps3/src/sys/sprx.s`) from `ps3/sprx.toml`.
3. Compile and link the chosen example package using `rustc` and `mold`.
4. Run `moldier patch` on the output ELF to resolve LV2 OPD descriptors and SPRX imports.
5. Produce `EBOOT.ELF` (and `EBOOT.BIN` if `make_fself` is available) in the project root.

## Running

### RPCS3 Emulator

You can run `EBOOT.ELF` directly in [RPCS3](https://rpcs3.net/).

### Real Hardware (PS3 with CFW / PS3HEN)

1. Place `EBOOT.BIN` inside a game directory on your PS3 internal HDD or USB drive:
   ```
   /dev_hdd0/game/HELLOCELL/USRDIR/EBOOT.BIN
   ```
2. Alternatively, mount and run directly via `webMAN MOD` or `/app_home/PS3_GAME/`.
3. Even more alternatively, use a Target Manager to upload the binary to your PS3 and run it.

## SPRX Dynamic Imports (`sprx.toml`)

`rust-ps3` allows defining Sony PRX module imports cleanly in `sprx.toml`. `moldier` parses this file, generates the appropriate assembly stubs, and binds them to the binary:

```toml
[libraries.cellSysmodule]
module_id = 0x0009
functions = [
    { name = "sysModuleLoad", fnid = 0x32267A31 },
    { name = "sysModuleUnload", fnid = 0x112A5EE9 },
]

[libraries.sys_net]
module_id = 0x0009
functions = [
    { name = "netInitializeNetworkEx", fnid = 0x139a9e9b },
    { name = "netSocket", fnid = 0x9c056962 },
    { name = "netConnect", fnid = 0x64f66d35 },
    { name = "netBind", fnid = 0xb0a59804 },
    { name = "netListen", fnid = 0x28e208bb },
    { name = "netAccept", fnid = 0xc94f6939 },
]
```

## Debugging

Debug symbols are preserved in `EBOOT.ELF`. You can inspect and debug binaries using:
- **RPCS3 Built-in Debugger / GDB Server**: Enable GDB debugging in RPCS3 settings and connect `gdb-multiarch` or `powerpc64-linux-gnu-gdb`.
- **TTY Output**: View real-time standard output and panic messages via `sys_tty_write` in the RPCS3 log window or PS3 Net Server.

### Useful tools

- [OpenTM](https://github.com/sagemono/opentm) - A fully open-source Target Manager for the PlayStation 3.
- [scetool](https://github.com/naehrwert/scetool) - A tool for signing and verifying PS3 EBOOTs and SPRXs.

## License

See [LICENSE](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.