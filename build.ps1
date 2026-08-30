$ErrorActionPreference = "Stop"

$BUILD = "target/powerpc-unknown-cellos/debug"

# 1. Build the moldier patcher tool (Host target).
$HOST_TARGET = (rustc -vV | Select-String "host: " | ForEach-Object { $_.Line.Substring(6) }).Trim()
cargo build -p moldier --target $HOST_TARGET
if ($LASTEXITCODE -ne 0) { throw "moldier build failed" }

# 2. Auto-generate SPRX assembly stubs from declarative sprx.toml.
cargo run -p moldier --target $HOST_TARGET -- gen-stubs --config sprx.toml --output src/sprx.s
if ($LASTEXITCODE -ne 0) { throw "SPRX stub generation failed" }

# 3. Build & link the binary directly using mold via rustc!
cargo +nightly build --target powerpc-unknown-cellos.json -Z unstable-options -Z build-std=core,alloc,compiler_builtins -Z json-target-spec -p hello-cell
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# 4. Process ELF with moldier (packs Sony LV2 OPD descriptors, dynamically binds SPRX headers, verifies ELF layout).
cargo run -p moldier --target $HOST_TARGET -- patch "$BUILD\EBOOT.ELF"
if ($LASTEXITCODE -ne 0) { throw "moldier patch failed" }

# 5. Package the ELF into an EBOOT.BIN.
$ABS_ELF = (Resolve-Path "$BUILD\EBOOT.ELF").Path
$ABS_BIN = "$((Resolve-Path $BUILD).Path)\EBOOT.BIN"

if (Get-Command "make_fself.exe" -ErrorAction SilentlyContinue) {
    make_fself.exe $ABS_ELF $ABS_BIN
} elseif (Get-Command "make_fself" -ErrorAction SilentlyContinue) {
    make_fself "$BUILD/EBOOT.ELF" "$BUILD/EBOOT.BIN"
}

Write-Host "Build complete with native mold + moldier: $ABS_BIN"


