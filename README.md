# PS3 Rust Barebones Starter

A minimal, working, barebones starting point for developing PlayStation 3 homebrew applications in **Rust**.

---

## Features

- **PPC64 ELFv1 ABI**: Built for PS3's native 64-bit PowerPC architecture with `.opd` function descriptors and stable `.TOC.` base addressing.
- **Dynamic Memory Allocation & Recycling (`alloc`)**: Powered by **Talc 5** with O(1) allocation and automatic deallocation/recycling, backed by dynamic kernel heap expansion via PS3 LV2 `sys_memory_allocate` (64 KB pages).
- **TTY Debug Output**: Standard `print!` and `println!` macros routed directly to the ProDG / TTY debug console via `SYS_TTY_WRITE`.
- **Safe LV2 Syscalls**: Inline assembly wrappers for PS3 Level 2 kernel syscalls with stack frame management and TOC (`r2`) register preservation.
- **Automated Toolchain**: PowerShell script (`build.ps1`) managing `cargo build`, cross-linking with PSL1GHT binutils in WSL, symbol stripping, and `make_fself` packaging.

---

## Project Structure

```
hello-cell/
├── .cargo/
│   └── config.toml          # Target rustflags
├── src/
│   ├── lib.rs               # Crate root and rust_main application logic
│   ├── entry.rs             # _start_code PowerPC64 ELFv1 entry point
│   ├── syscalls.rs          # PS3 LV2 kernel syscalls (TTY, Memory, Timer, Exit)
│   ├── allocator.rs         # Ps3Heap GlobalAlloc implementation
│   ├── io.rs                # print! and println! macros over TTY
│   ├── mem.rs               # C runtime intrinsics (memcpy, memset, memcmp, etc.)
│   └── panic.rs             # Custom panic handler reporting to TTY
├── powerpc-unknown-cellos.json  # Custom Rust target specification (ELFv1 PPC64 BE)
├── ps3.ld                   # GNU LD linker script with .opd and TOC layout
├── build.ps1                # Build, link, and packaging script
└── Cargo.toml               # Cargo package configuration
```

---

## Prerequisites

> [!IMPORTANT]
> PSL1GHT is required because of its ported GNU Binutils linker. Rust's linker is unable to produce PS3-compatible ELFv1 binaries with `.opd` function descriptors. The build script automates the cross-linking process using PSL1GHT's `powerpc64-ps3-elf-ld` in WSL.

1. **Rust Nightly** on Windows:
   ```powershell
   rustup toolchain install nightly
   rustup component add rust-src --toolchain nightly
   ```
2. **PSL1GHT / PS3 Toolchain** in WSL (Arch Linux or Ubuntu):
   - `powerpc64-ps3-elf-ld` (GNU Binutils 2.22+)
   - `powerpc64-ps3-elf-strip`
3. **`make_fself.exe`** from the PS3 SDK available in PATH (or root directory).

---

## Building

Run the automated build script in PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\build.ps1
```

This will:
1. Compile `core`, `alloc`, and `compiler_builtins` for `powerpc-unknown-cellos` via `cargo +nightly build`.
2. Link the static library into `target/.../EBOOT.ELF` using `powerpc64-ps3-elf-ld` inside WSL.
3. Strip DWARF symbols with `powerpc64-ps3-elf-strip`.
4. Package into `target/.../EBOOT.BIN` with `make_fself.exe`.

---

## Running / Debugging

- **Hardware / ProDG**: Load `EBOOT.BIN` or `EBOOT.ELF` into ProDG Debugger or launch via Target Manager.
- **RPCS3**: Load `EBOOT.BIN` as a PS3 executable.

You should see output similar to:
```
========================================
 Hello PlayStation 3 from barebones Rust!
========================================
Boxed Value: 0x13374242
Vector Items: [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]
Vector Length: 10
Formatted string dynamically: 0xCAFEBABE
All allocations and formatting succeeded!
Heartbeat tick: 1
Heartbeat tick: 2
...
```

---

## Technical Notes

### PowerPC64 ELFv1 vs ELFv2 ABI
PlayStation 3 CellOS (LV2) is natively built on the **PowerPC64 ELFv1 ABI**:
- Function addresses in symbol tables point to **Function Descriptors** in `.opd` (24 bytes: `{entry_point, toc_base, env_pointer}`).
- Register `r2` holds the `.TOC.` (Table of Contents) base pointer and is invariant across direct function calls.
- In contrast, ELFv2 uses dual Global/Local Entry Points (GEP/LEP) which require modern linkers (Binutils 2.24+) to adjust direct branch offsets. Using ELFv1 ensures 100% compatibility with standard PS3 Binutils toolchains.

### Syscall Stack Frames
PS3 LV2 kernel syscalls (`sc 2`) expect a minimum 112-byte ABI linkage stack frame. All syscall wrappers create a temporary 128-byte frame (`stdu 1, -128(1)`) and preserve non-volatile registers before executing `sc 2`.

---

## License

AGPLv3.0
