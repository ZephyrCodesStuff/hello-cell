#!/usr/bin/env bash
set -euo pipefail

HOST_TARGET=$(rustc -vV | sed -n 's|host: ||p')
BUILD_DIR="target/powerpc-unknown-cellos/debug"
ELF_PATH="$BUILD_DIR/EBOOT.ELF"
BIN_PATH="$BUILD_DIR/EBOOT.BIN"
MOLDIER_BIN="target/$HOST_TARGET/debug/moldier"

echo "===> 1. Building moldier (Host: $HOST_TARGET)..."
cargo build -p moldier --target "$HOST_TARGET"

echo "===> 2. Building hello-cell (PS3 PPC64 ELFv1 Staticlib)..."
cargo +nightly build \
    --target powerpc-unknown-cellos.json \
    -Z unstable-options \
    -Z build-std=core,alloc,compiler_builtins \
    -Z json-target-spec \
    -p hello-cell

echo "===> 3. Linking with mold (PPC64 ELFv1)..."
mold -m elf64ppc \
    --image-base 0x10000 \
    --no-rosegment \
    -z norelro \
    -z separate-loadable-segments \
    -Bstatic \
    -e _start_code \
    --whole-archive "$BUILD_DIR/libhello_cell.a" \
    --no-whole-archive \
    -o "$ELF_PATH"

echo "===> 4. Patching Sony LV2 OPD descriptors & SPRX stubs with moldier..."
"$MOLDIER_BIN" patch "$ELF_PATH"

echo "===> 5. Packaging into EBOOT.BIN..."
if command -v make_fself &> /dev/null; then
    make_fself "$ELF_PATH" "$BIN_PATH"
    echo "✓ Build complete: $BIN_PATH"
elif command -v make_fself.exe &> /dev/null; then
    make_fself.exe "$ELF_PATH" "$BIN_PATH"
    echo "✓ Build complete: $BIN_PATH"
else
    echo "Note: 'make_fself' not found in PATH. EBOOT.ELF is ready at $ELF_PATH."
fi
