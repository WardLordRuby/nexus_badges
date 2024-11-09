#!/bin/bash
set -e

# Change to project root (parent of the current script's directory)
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"
cd "$PROJECT_ROOT"

# Default values
BINARY_NAME="nexus_badges"
TARGET="x86_64-apple-darwin"
VERSION=""

# Parse command line arguments
usage() {
    echo "Usage: $0 [-b binary_name](default: nexus_badges) [-t target](default: x86_64-apple-darwin) [-v version](required)"
    exit 1
}

while getopts "b:t:v:h" opt; do
    case $opt in
        b) BINARY_NAME="$OPTARG";;
        t) TARGET="$OPTARG";;
        v) VERSION="$OPTARG";;
        h|?) usage;;
    esac
done

# Check if version is provided
if [ -z "$VERSION" ]; then
    echo "Error: Version parameter is required"
    usage
fi

# Format the app name
APP_NAME=$(echo "$BINARY_NAME" | sed 's/_/-/g')
APP_NAME_DISPLAY=$(echo "$BINARY_NAME" | sed 's/[_-]/ /g' | awk '{for(i=1;i<=NF;i++)sub(/./,toupper(substr($i,1,1)),$i)}1')
PKG_NAME="${APP_NAME}-${VERSION}"

# Build release version
echo "Building release version for $TARGET..."
cargo build --release --target "$TARGET"

# Create distribution directory structure
DIST_DIR="build/tmp/dist"
PAYLOAD_DIR="$DIST_DIR/payload"
rm -rf "build/tmp"
mkdir -p "$PAYLOAD_DIR"

# Copy binary to payload directory
cp "target/$TARGET/release/$BINARY_NAME" "$PAYLOAD_DIR/$APP_NAME"
chmod +x "$PAYLOAD_DIR/$APP_NAME"

# Copy icon and other resources to dist/Resources
mkdir -p "$DIST_DIR/Resources"
cp "assets/Icon.png" "$DIST_DIR/Resources/"
sips -z 220 220 --padToHeightWidth 275 250 "$DIST_DIR/Resources/Icon.png"

cat > "$DIST_DIR/Resources/conclusion.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=utf-8">
    <style>
        body { 
            font-family: -apple-system;
            margin: 0;
            padding: 20px;
        }
        .custom-message {
            margin-top: 120px;
            padding: 10px 20px;
        }
        h1 {
            font-size: 21px;
            margin-bottom: 20px;
            text-align: center;
        }
    </style>
</head>
<body>
    <h1>The installation was successful.</h1>
    
    <div class="custom-message">
        <p>$APP_NAME_DISPLAY was successfully installed. You can now use the app from the command line.</p>
        <p>For help use '$APP_NAME --help' or refer to the documentation at <a href="https://github.com/WardLordRuby/nexus_badges">github.com/WardLordRuby/nexus_badges</a>.</p>
    </div>
</body>
</html>
EOF

# Create distribution.xml
cat > "$DIST_DIR/distribution.xml" << EOF
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="1">
    <title>$APP_NAME_DISPLAY</title>
    <background file="Icon.png" mime-type="image/png" alignment="bottomleft"/>
    <background-darkAqua file="Icon.png" mime-type="image/png" alignment="bottomleft"/>
    <options require-scripts="false"/>
    <choices-outline>
        <line choice="default"/>
    </choices-outline>
    <choice id="default" visible="false" selected="true" title="$APP_NAME_DISPLAY">
        <pkg-ref id="com.$APP_NAME.pkg"/>
    </choice>
    <pkg-ref id="com.$APP_NAME.pkg" version="$VERSION">$PKG_NAME.pkg</pkg-ref>
    <conclusion file="conclusion.html" mime-type="text/html"/>
</installer-gui-script>
EOF

# Create package installer
pkgbuild \
  --identifier "com.$APP_NAME.pkg" \
  --root "$PAYLOAD_DIR" \
  --install-location "/usr/local/bin" \
  "$DIST_DIR/$PKG_NAME.pkg"

productbuild \
  --distribution "$DIST_DIR/distribution.xml" \
  --resources "$DIST_DIR/Resources" \
  --package-path "$DIST_DIR" \
  "$DIST_DIR/$APP_NAME.pkg"

# Create DMG
echo "Creating DMG..."
hdiutil create -volname "$APP_NAME_DISPLAY" -srcfolder "$DIST_DIR/$APP_NAME.pkg" -ov -format UDZO "target/$TARGET/release/$PKG_NAME.dmg"

rm -rf "build/tmp"

echo "Done! Created target/$TARGET/release$PKG_NAME.dmg"