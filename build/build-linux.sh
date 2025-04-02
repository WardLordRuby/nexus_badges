#!/bin/bash
set -e

# Default values
BINARY_NAME="nexus_badges"
TARGET="x86_64-unknown-linux-gnu"

usage() {
    echo "Usage: $0 [-b binary_name](default: nexus_badges) [-t target](default: x86_64-unknown-linux-gnu) [-v version](required)"
    exit 1
}

# Parse command line arguments
while getopts "b:t:v:h" opt; do
    case $opt in
        b) BINARY_NAME="$OPTARG";;
        t) TARGET="$OPTARG";;
        v) VERSION="$OPTARG";;
        h|?) usage;;
    esac
done

# Ensure VERSION is set
if [ -z "$VERSION" ]; then
  echo "Error: --version is required"
  exit 1
fi

# Define metadata
case "$TARGET" in
  "x86_64-unknown-linux-gnu") ARCHITECTURE="amd64" ;;
  "aarch64-unknown-linux-gnu") ARCHITECTURE="arm64" ;;
  *)
    echo "Error: unsupported target"
    exit 1
    ;;
esac

# Check if script is run as root
if [[ $EUID -ne 0 ]]; then
    echo "This script requires superuser privileges. Asking for sudo..."
    exec sudo -E "$0" "$@"
fi

# Ensure BinaryName is Linux-friendly
LINUX_BINARY_NAME="$(echo "$BINARY_NAME" | tr '_' '-')"

# Save the current directory to the stack
pushd . > /dev/null

# Ensure script always has the same working directory
cd "$(dirname "$0")" || exit 1

# Change to project root
cd ..

# Build binary in release mode for desired target
cargo build --target "$TARGET" --release
if [ $? -ne 0 ]; then exit 1; fi

# Check if debforge is installed
if command -v debforge > /dev/null 2>&1; then
    debforge --binary-name "$BINARY_NAME" --version "$VERSION" --target "$TARGET"
else
    if [ ! -f "build/debforge" ]; then
        # Download debforge
        git clone https://github.com/WardLordRuby/debforge.git build/debforge-repo
        cargo build --release --manifest-path build/debforge-repo/Cargo.toml
        if [ $? -ne 0 ]; then exit 1; fi
        mv build/debforge-repo/target/release/debforge build/debforge
        rm -rf build/debforge-repo
    fi
    ./build/debforge --binary-name "$BINARY_NAME" --version "$VERSION" --target "$TARGET"
fi

# Run our debian package builder script
./build/debian/build-deb.sh build/tmp/dist/linux

mv -f "build/tmp/dist/linux/${LINUX_BINARY_NAME}-${VERSION}.deb" \
      "target/${TARGET}/release/${BINARY_NAME}_linux_${ARCHITECTURE}.deb"

# Delete tmp directory
rm -rf build/tmp

# Revert back to the original directory
popd > /dev/null

echo "Done! Package created in target/${TARGET}/release/"

