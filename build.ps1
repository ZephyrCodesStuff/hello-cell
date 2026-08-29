$ErrorActionPreference = "Stop"

$WSL_DISTRO    = if ($env:WSL_DISTRO) { $env:WSL_DISTRO } else { "archlinux" }
$PS3DEV        = if ($env:PS3DEV) { $env:PS3DEV } else { "/home/zeph/Coding/ps3dev" }
$PSL1GHT_AS    = "$PS3DEV/ppu/bin/powerpc64-ps3-elf-as"
$PSL1GHT_LD    = "$PS3DEV/ppu/bin/powerpc64-ps3-elf-ld"
$PSL1GHT_STRIP = "$PS3DEV/ppu/bin/powerpc64-ps3-elf-strip"
$BUILD         = "target/powerpc-unknown-cellos/debug"

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

# 2. Assemble SPRX FNID stubs and link inside WSL with PSL1GHT linker.
$W_BUILD = WinToWsl((Resolve-Path $BUILD))
$W_LD    = WinToWsl((Resolve-Path "ps3.ld"))
$W_SPRX  = WinToWsl((Resolve-Path "src/sprx.s"))
$W_OUT   = "$W_BUILD/EBOOT.ELF"

wsl -d $WSL_DISTRO -e bash -lc "$PSL1GHT_AS -mregnames $W_SPRX -o $W_BUILD/sprx.o"
if ($LASTEXITCODE -ne 0) { throw "sprx assembly failed" }

wsl -d $WSL_DISTRO -e bash -lc "$PSL1GHT_LD -m elf64ppc -T $W_LD -o $W_OUT $W_BUILD/sprx.o -L$W_BUILD --whole-archive $W_BUILD/libhello_cell.a --no-whole-archive"
if ($LASTEXITCODE -ne 0) { throw "link failed" }

# 3. Strip debug symbols for clean ProDG execution.
wsl -d $WSL_DISTRO -e bash -lc "$PSL1GHT_STRIP -g $W_OUT"
if ($LASTEXITCODE -ne 0) { throw "strip failed" }

# 4. Process ELF with PSL1GHT-compatible sprxlinker (sets OSABI, verifies FNID imports, packs OPD descriptors).
python sprxlinker.py "$BUILD\EBOOT.ELF"
if ($LASTEXITCODE -ne 0) { throw "sprxlinker failed" }

# 5. Package the ELF into an EBOOT.BIN.
$BIN_PATH = "$BUILD\EBOOT.BIN"

make_fself.exe "$BUILD\EBOOT.ELF" $BIN_PATH
if ($LASTEXITCODE -ne 0) { throw "make_fself failed" }

Write-Host "Build complete with PSL1GHT FNID networking: $BIN_PATH"
