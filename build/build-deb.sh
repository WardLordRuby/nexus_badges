#!/bin/bash
set -e

# Check if package directory exists
if [ -z "$(ls -A /build/temp/dist/linux)" ]; then
    echo "Error: No package found in /build/temp/dist/linux"
    exit 1
fi

# Find the package directory
PACKAGE_DIR=$(ls -d /build/temp/dist/linux/*/ | head -n 1)

# Set permissions
chmod 755 "$PACKAGE_DIR/DEBIAN"
chmod 755 "$PACKAGE_DIR/DEBIAN/postrm"
chmod 644 "$PACKAGE_DIR/DEBIAN/control"

# Build the package
dpkg-deb --build "$PACKAGE_DIR"

echo "Package built successfully!"