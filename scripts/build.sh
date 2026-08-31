#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

TARGET_EXAMPLE="${1:-hello-world}"

case "$TARGET_EXAMPLE" in
    hello-world|example-hello-world)
        PACKAGE_NAME="example-hello-world"
        ;;
    http-server|example-http-server)
        PACKAGE_NAME="example-http-server"
        ;;
    *)
        PACKAGE_NAME="$TARGET_EXAMPLE"
        ;;
esac

HOST_TARGET=$(rustc -vV | sed -n 's|host: ||p')
BUILD_DIR="target/powerpc64-sony-ps3/debug"
ELF_PATH="$BUILD_DIR/EBOOT.ELF"
BIN_PATH="$BUILD_DIR/EBOOT.BIN"
MOLDIER_BIN="target/$HOST_TARGET/debug/moldier"

echo "===> 1. Building moldier (Host: $HOST_TARGET)..."
cargo build --manifest-path moldier/Cargo.toml --target-dir target --target "$HOST_TARGET"

echo "===> 2. Generating SPRX assembly stubs from ps3/sprx.toml..."
"$MOLDIER_BIN" gen-stubs --config ps3/sprx.toml --output ps3/src/sys/sprx.s

echo "===> 3. Building & linking '$PACKAGE_NAME' with mold..."
cargo +nightly build \
    --target powerpc64-sony-ps3.json \
    -Z unstable-options \
    -Z build-std=core,alloc,compiler_builtins \
    -Z json-target-spec \
    -p "$PACKAGE_NAME"

echo "===> 4. Patching Sony LV2 OPD descriptors & SPRX stubs with moldier..."
"$MOLDIER_BIN" patch "$ELF_PATH"

echo "===> 5. Packaging into EBOOT.BIN..."
if command -v make_fself &> /dev/null; then
    make_fself "$ELF_PATH" "$BIN_PATH"
elif command -v make_fself.exe &> /dev/null; then
    make_fself.exe "$ELF_PATH" "$BIN_PATH"
else
    echo "Note: 'make_fself' not found in PATH. EBOOT.ELF is ready at $ELF_PATH."
fi

if [ -f "$BIN_PATH" ]; then
    echo "✓ Build complete ($PACKAGE_NAME): ./EBOOT.BIN"
else
    echo "✓ Build complete ($PACKAGE_NAME): ./EBOOT.ELF"
fi
