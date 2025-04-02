#!/bin/bash
set -e

DIST="/build/tmp/dist/linux"

# Check if dist is overwritten
if [[ -n "$1" && "$1" != -* ]]; then
    DIST="$1"
fi

# Check if dist directory exists
if [ -z "$(ls -A "$DIST")" ]; then
    echo "Error: linux distribution directory: "$DIST", not found"
    exit 1
fi

# Find desired directories
PACKAGE_DIR=$(ls -d "$DIST/"* | head -n 1)
PACKAGE=$(basename "$PACKAGE_DIR")
LINUX_BINARY_NAME=${PACKAGE%-*}
CHANGELOG_DIR="$PACKAGE_DIR/usr/share/doc/$LINUX_BINARY_NAME"

# Compress changelog
gzip -n -9 "$CHANGELOG_DIR/changelog"

# Set permissions
chmod 755 -R "$PACKAGE_DIR"
chmod 644 "$PACKAGE_DIR/DEBIAN/control"
chmod 644 "$PACKAGE_DIR/usr/share/applications/$LINUX_BINARY_NAME.desktop"
chmod 644 "$PACKAGE_DIR/usr/share/doc/$LINUX_BINARY_NAME/copyright"
chmod 644 "$PACKAGE_DIR/usr/share/icons/hicolor/256x256/apps/$LINUX_BINARY_NAME.png"
chmod 644 "$PACKAGE_DIR/usr/share/icons/hicolor/512x512/apps/$LINUX_BINARY_NAME.png"
chmod 644 "$CHANGELOG_DIR/changelog.gz"
chown root:root "$PACKAGE_DIR/usr/local/bin/$LINUX_BINARY_NAME"

# Build the package
dpkg-deb --build "$PACKAGE_DIR"

echo "Package built successfully!"