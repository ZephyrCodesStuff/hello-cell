$ErrorActionPreference = "Stop"

$WSL_DISTRO    = if ($env:WSL_DISTRO) { $env:WSL_DISTRO } else { "archlinux" }
$BUILD         = "target/powerpc-unknown-cellos/debug"

# Translate a Windows path to its WSL mount path (e.g. C:\foo -> /mnt/c/foo)
function WinToWsl($path) {
    $p = $path.ToString()
    $drive = $p.Substring(0, 1).ToLower()
    $rest  = $p.Substring(2).Replace('\', '/')
    return "/mnt/$drive$rest"
}

# 1. Build the moldier patcher tool (Host target).
$HOST_TARGET = (rustc -vV | Select-String "host: " | ForEach-Object { $_.Line.Substring(6) }).Trim()
cargo build -p moldier --target $HOST_TARGET
if ($LASTEXITCODE -ne 0) { throw "moldier build failed" }

# 2. Auto-generate SPRX assembly stubs from declarative sprx.toml.
cargo run -p moldier --target $HOST_TARGET -- gen-stubs --config sprx.toml --output src/sprx.s
if ($LASTEXITCODE -ne 0) { throw "SPRX stub generation failed" }

# 3. Build the Rust staticlib (ELFv1 PPC64 BE).
cargo +nightly build --target powerpc-unknown-cellos.json -Z unstable-options -Z build-std=core,alloc,compiler_builtins -Z json-target-spec -p hello-cell
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# 4. Link with mold (ultra-fast PPC64 ELFv1 linker).
$W_BUILD = WinToWsl((Resolve-Path $BUILD))
$W_OUT   = "$W_BUILD/EBOOT.ELF"

if (Get-Command "mold" -ErrorAction SilentlyContinue) {
    mold -m elf64ppc --image-base 0x10000 --no-rosegment -z norelro -z separate-loadable-segments -Bstatic -e _start_code --whole-archive "$BUILD/libhello_cell.a" --no-whole-archive -o "$BUILD/EBOOT.ELF"
} else {
    wsl -d $WSL_DISTRO -e bash -lc "mold -m elf64ppc --image-base 0x10000 --no-rosegment -z norelro -z separate-loadable-segments -Bstatic -e _start_code --whole-archive $W_BUILD/libhello_cell.a --no-whole-archive -o $W_OUT"
}
if ($LASTEXITCODE -ne 0) { throw "mold link failed" }

# 5. Process ELF with moldier (packs Sony LV2 OPD descriptors, dynamically binds SPRX headers, verifies ELF layout).
cargo run -p moldier --target $HOST_TARGET -- patch "$BUILD\EBOOT.ELF"
if ($LASTEXITCODE -ne 0) { throw "moldier patch failed" }

# 6. Package the ELF into an EBOOT.BIN.
$ABS_ELF = (Resolve-Path "$BUILD\EBOOT.ELF").Path
$ABS_BIN = "$((Resolve-Path $BUILD).Path)\EBOOT.BIN"

if (Get-Command "make_fself.exe" -ErrorAction SilentlyContinue) {
    make_fself.exe $ABS_ELF $ABS_BIN
} elseif (Get-Command "make_fself" -ErrorAction SilentlyContinue) {
    make_fself "$BUILD/EBOOT.ELF" "$BUILD/EBOOT.BIN"
}

# Copy finalized binaries to project root
Copy-Item "$BUILD\EBOOT.ELF" "EBOOT.ELF" -Force
Copy-Item "$BUILD\EBOOT.BIN" "EBOOT.BIN" -Force

Write-Host "Build complete with mold + moldier: $BIN_PATH"

