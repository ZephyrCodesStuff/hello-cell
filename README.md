# hello-cell: Bare-Metal Rust on PlayStation 3 (Cell B.E.)

An experimental research prototype and proof-of-concept demonstrating modern, native `#![no_std]` **Rust** executing directly on the **PlayStation 3's Cell Broadband Engine (PPE)** under the GameOS LV2 kernel.

---

> [!NOTE]
> This is a **research and experimental prototype** exploring ABI translation, kernel syscall boundaries, Talc dynamic memory allocation, and PSL1GHT SPRX linking for the PS3 PPE. It is not yet a complete standard library (`std`).

---

## Current Status & What Works

- [x] **PPC64 ELFv1 ABI**: Custom target JSON (`powerpc-unknown-cellos.json`) generating 64-bit Big-Endian PowerPC code with stable `.TOC.` addressing.
- [x] **Kernel Bootstrap**: Linker script (`ps3.ld`) constructing the LV2 kernel 24-byte `_start` OPD descriptor (`[PC, TOC, ENV]`) and enforcing 64 KB segment alignment (`PHDRS`).
- [x] **Dynamic Memory (`alloc`)**: O(1) heap allocation powered by **Talc 5.1** implementing a dynamic `Source` provider that fetches $\ge 4\text{ MB}$ chunks on demand from LV2 `sys_memory_allocate`.
- [x] **Console TTY Output**: Formatted string printing (`print!`, `println!`) routed through `SYS_TTY_WRITE` (Syscall 403).
- [x] **Kernel Syscall ABI**: Inline assembly wrappers managing 128-byte stack linkage frames and TOC register (`r2`) preservation.
- [ ] **SPRX Dynamic Linking / Networking (WIP)**: Experimental PSL1GHT FNID import tables (`src/sprx.s`) and post-link ELF patching (`sprxlinker.py`) for `cellSysmodule` and `sys_net`.

---

## Project Structure

```
hello-cell/
├── .cargo/
│   └── config.toml          # Target rustflags (-C dwarf-version=2)
├── src/
│   ├── lib.rs               # Crate root and rust_main application logic
│   ├── entry.rs             # _start_code PowerPC64 ELFv1 bootstrap entry point
│   ├── syscalls.rs          # PS3 LV2 kernel syscalls (TTY, Memory, Timer, PRX, Exit)
│   ├── allocator.rs         # Talc 5.1 dynamic Source allocator & Ps3RawMutex
│   ├── io.rs                # print! and println! macros over TTY
│   ├── mem.rs               # C runtime intrinsics (memcpy, memset, memcmp, memmove, strlen)
│   ├── net.rs               # Experimental socket abstractions & sys_net wrappers
│   ├── panic.rs             # Custom panic handler reporting to TTY
│   └── sprx.s               # SPRX parameters, FNID lookup tables, and OPD stubs
├── powerpc-unknown-cellos.json  # Custom Rust target specification (PPC64 BE ELFv1)
├── ps3.ld                   # GNU LD linker script with 64KB page alignment & .opd layout
├── sprxlinker.py            # ELF post-processor for PSL1GHT PRX structures
├── build.ps1                # Build, assemble, link, strip, and packaging pipeline
└── Cargo.toml               # Cargo package configuration
```

---

## Prerequisites

1. **Rust Nightly**:
   ```powershell
   rustup toolchain install nightly
   rustup component add rust-src --toolchain nightly
   ```
2. **PSL1GHT / PS3 Binutils** in WSL (Arch Linux or Ubuntu):
   - `powerpc64-ps3-elf-as`
   - `powerpc64-ps3-elf-ld` (GNU Binutils 2.22+)
   - `powerpc64-ps3-elf-strip`
3. **Python 3** (for `sprxlinker.py` ELF post-processing).
4. **`make_fself.exe`** from the PS3 SDK in PATH (or workspace root).

---

## Building

Run the automated PowerShell build script:

```powershell
powershell -ExecutionPolicy Bypass -File .\build.ps1
```

The pipeline performs the following steps:
1. Compiles `core`, `alloc`, and `compiler_builtins` for `powerpc-unknown-cellos` via `cargo +nightly build`.
2. Assembles SPRX FNID stubs (`src/sprx.s`) using `powerpc64-ps3-elf-as` in WSL.
3. Links the static library with `powerpc64-ps3-elf-ld -T ps3.ld`.
4. Strips debug symbols with `powerpc64-ps3-elf-strip -g`.
5. Post-processes the ELF with `sprxlinker.py` to fix `.lib.stub` import counts and pack `.opd` descriptors.
6. Packages `EBOOT.ELF` into `EBOOT.BIN` via `make_fself.exe`.

---

## Running & Debugging

- **Hardware (DEX / DECR)**: Launch via Target Manager / ProDG Debugger or open-source [OpenTM](https://github.com/sagemono/OpenTM) over Ethernet.
- **RPCS3**: Boot `EBOOT.BIN` or `EBOOT.ELF` directly.

Sample TTY output:
```text
========================================
 Hello PlayStation 3 from barebones Rust!
========================================
Boxed Value: 0x13374242
Vector Items: [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]
Vector Length: 10
Formatted string dynamically: 0xCAFEBABE
All allocations and formatting succeeded!
--- Heap Stats BEFORE Temp Allocations ---
  Active Allocs: 2
  Active Bytes:  56 bytes
  Claimed Total: 4 MB
Allocating a 64KB vector (8000 u64 elements)...
--- Heap Stats WHILE Vector is Live ---
  Active Allocs: 3
  Active Bytes:  64056 bytes (+64000)
  Vector capacity in bytes: 64000
--- Heap Stats AFTER Vector Dropped (Deallocated!) ---
  Active Allocs: 2
  Active Bytes:  56 bytes (Freed back to Talc!)
  Total Lifetime Allocs: 3
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
- In contrast, ELFv2 uses dual Global/Local Entry Points (GEP/LEP) which require modern linkers (Binutils 2.24+) to adjust direct branch offsets. Using pure ELFv1 ensures 100% compatibility with standard PS3 Binutils toolchains.

### Syscall Stack Frames
PS3 LV2 kernel syscalls expect a minimum 112-byte ABI linkage stack frame. All syscall wrappers create a temporary 128-byte frame (`stdu 1, -128(1)`) and preserve the TOC register `r2` into `r22` before executing `sc`.

---

## License

AGPL-3.0
