$ErrorActionPreference = "Stop"

$WSL_DISTRO   = "archlinux"
$PSL1GHT_LD  = "/home/zeph/Coding/ps3dev/ppu/bin/powerpc64-ps3-elf-ld"
$PSL1GHT_STRIP = "/home/zeph/Coding/ps3dev/ppu/bin/powerpc64-ps3-elf-strip"
$BUILD       = "target/powerpc-unknown-cellos/debug"

# Translate a Windows path to its WSL mount path (e.g. C:\foo -> /mnt/c/foo)
function WinToWsl($path) {
    $p = $path.ToString()
    $drive = $p.Substring(0, 1).ToLower()
    $rest  = $p.Substring(2).Replace('\', '/')
    return "/mnt/$drive$rest"
}

# 1. Build the Rust staticlib (ELFv1 PPC64 BE).
cargo +nightly build --target powerpc-unknown-cellos.json -Z unstable-options -Z build-std=core,alloc,compiler_builtins -Z json-target-spec
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# 2. Link inside WSL with the psl1ght linker, using our PS3 linker script.
#    Everything else (compilation, final self packaging) happens in Windows.
$W_BUILD = WinToWsl((Resolve-Path $BUILD))
$W_LD    = WinToWsl((Resolve-Path "ps3.ld"))
$W_OUT   = "$W_BUILD/EBOOT.ELF"

wsl -d $WSL_DISTRO -e bash -lc "$PSL1GHT_LD -m elf64ppc -T $W_LD -o $W_OUT -L$W_BUILD --whole-archive $W_BUILD/libhello_cell.a --no-whole-archive"
if ($LASTEXITCODE -ne 0) { throw "link failed" }

# 3. Package the ELF into an EBOOT.BIN (Windows tool from the old PS3 SDK).
$BIN_PATH = "$BUILD\EBOOT.BIN"

# Strip all debug symbols from the ELF before packaging, since ProDG won't read DWARF 2 with 8-byte addresses.
wsl -d $WSL_DISTRO -e bash -lc "$PSL1GHT_STRIP -g $W_OUT"
if ($LASTEXITCODE -ne 0) { throw "strip failed" }

make_fself.exe "$BUILD\EBOOT.ELF" $BIN_PATH
if ($LASTEXITCODE -ne 0) { throw "make_fself failed" }

Write-Host "Build complete: $BIN_PATH"
