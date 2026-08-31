param (
    [string]$TargetExample = "hello-world"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
Set-Location $RootDir

switch ($TargetExample) {
    "hello-world" { $PackageName = "example-hello-world" }
    "example-hello-world" { $PackageName = "example-hello-world" }
    "http-server" { $PackageName = "example-http-server" }
    "example-http-server" { $PackageName = "example-http-server" }
    Default { $PackageName = $TargetExample }
}

$BUILD = "target/powerpc64-sony-ps3/debug"

# 1. Build the moldier patcher tool (Host target).
$HOST_TARGET = (rustc -vV | Select-String "host: " | ForEach-Object { $_.Line.Substring(6) }).Trim()
cargo build --manifest-path moldier/Cargo.toml --target-dir target --target $HOST_TARGET
if ($LASTEXITCODE -ne 0) { throw "moldier build failed" }

# 2. Auto-generate SPRX assembly stubs from declarative sprx.toml.
$MOLDIER_BIN = "target/$HOST_TARGET/debug/moldier"
& $MOLDIER_BIN gen-stubs --config ps3/sprx.toml --output ps3/src/sys/sprx.s
if ($LASTEXITCODE -ne 0) { throw "SPRX stub generation failed" }

# 3. Build & link the binary directly using mold via rustc!
cargo +nightly build --target powerpc64-sony-ps3.json -Z unstable-options -Z build-std=core,alloc,compiler_builtins -Z json-target-spec -p $PackageName
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# 4. Process ELF with moldier (packs Sony LV2 OPD descriptors, dynamically binds SPRX headers, verifies ELF layout).
& $MOLDIER_BIN patch "$BUILD\EBOOT.ELF"
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
