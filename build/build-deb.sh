#!/bin/bash
set -e

# Check if package directory exists
if [ -z "$(ls -A /build/tmp/dist/linux)" ]; then
    echo "Error: No package found in /build/tmp/dist/linux"
    exit 1
fi

# Find desired directories
PACKAGE_DIR=$(ls -d /build/tmp/dist/linux/* | head -n 1)
PACKAGE=$(basename "$PACKAGE_DIR")
PACKAGE_NAME=${PACKAGE%-*}
CHANGELOG_DIR="$PACKAGE_DIR/usr/share/doc/$PACKAGE_NAME"
DESKTOP_FILE="$PACKAGE_DIR/usr/share/applications/$PACKAGE_NAME.desktop"
CONTROL_FILE="$PACKAGE_DIR/DEBIAN/control"

# Ensure LF line endings
dos2unix "$DESKTOP_FILE"
dos2unix "$CONTROL_FILE"
dos2unix "$CHANGELOG_DIR/changelog"
dos2unix "$PACKAGE_DIR/DEBIAN/postrm"

# Compress changelog
gzip -n -9 "$CHANGELOG_DIR/changelog"

# Set permissions
chmod 755 -R "$PACKAGE_DIR"
chmod 644 "$CONTROL_FILE"
chmod 644 "$DESKTOP_FILE"
chmod 644 "$PACKAGE_DIR/usr/share/icons/hicolor/256x256/apps/$PACKAGE_NAME.png"
chmod 644 "$CHANGELOG_DIR/changelog.gz"
chown root:root "$PACKAGE_DIR/usr/local/bin/$PACKAGE_NAME"

# Build the package
dpkg-deb --build "$PACKAGE_DIR"

echo "Package built successfully!"