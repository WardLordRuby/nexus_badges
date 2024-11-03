param(
    [string]$BinaryName = "nexus_badges",
    [string]$Target = "x86_64-unknown-linux-gnu",
    [Parameter(Mandatory = $true)]
    [string]$Version
)

# Get project root directory
$ProjectRoot = Split-Path -Parent $PSScriptRoot

# Ensure BinaryName is linux friendly
$LinuxBinaryName = $BinaryName.Replace('_', '-')

# Define metadata
if ($Target -eq "x86_64-unknown-linux-gnu") {
    $Architecture = "amd64"
} elseif ($Target -eq "aarch64-unknown-linux-gnu") {
    $Architecture = "arm64"
} else {
    Write-Error "unsupported target"
    exit 1
}

$DesktopFile = @"
[Desktop Entry]
Version=$Version
Type=Application
Name=$LinuxBinaryName
Comment=Your application description
Exec=/usr/local/bin/$LinuxBinaryName
Icon=$LinuxBinaryName
Categories=Utility;
Terminal=true
"@

$ControlFile = @"
Package: $LinuxBinaryName
Version: $Version
Section: utils
Priority: optional
Architecture: $Architecture
Maintainer: WardLordRuby
Description: Shields.io badge generator for Nexus Mods
"@

# Change to project root
Push-Location $ProjectRoot

try {
    cross build --target $Target --release

    # Create distribution directory
    $DistDir = "build/temp/dist/linux/$LinuxBinaryName-$Version"
    New-Item -ItemType Directory -Force -Path $DistDir
    New-Item -ItemType Directory -Force -Path "$DistDir/usr/local/bin"
    New-Item -ItemType Directory -Force -Path "$DistDir/usr/share/applications"
    New-Item -ItemType Directory -Force -Path "$DistDir/usr/share/icons/hicolor/256x256/apps"

    # Copy binary
    Copy-Item "target/$Target/release/$BinaryName" `
        "$DistDir/usr/local/bin/$LinuxBinaryName"

    # Copy icon
    Copy-Item "assets/Icon_256.png" `
        "$DistDir/usr/share/icons/hicolor/256x256/apps/$LinuxBinaryName.png"

    # Create .desktop file
    $DesktopFile | Out-File -FilePath "$DistDir/usr/share/applications/$LinuxBinaryName.desktop" -Encoding UTF8

    # Create control file for debian package
    New-Item -ItemType Directory -Force -Path "$DistDir/DEBIAN"
    $ControlFile | Out-File -FilePath "$DistDir/DEBIAN/control" -Encoding UTF8

    # Build the Docker image if it doesn't exist
    $ImageName = "deb-builder"
    if (-not (docker images -q $ImageName)) {
        Write-Host "Building Docker image..."
        docker build -t $ImageName ./build/.
    }

    # Run the container
    if (docker run --rm -v "${PWD}/build/temp:/build/temp" $ImageName -and $?) {
        Write-Host "Done! Package created in target/$Target/release/"

        Move-Item "build/temp/dist/linux/$LinuxBinaryName-$Version.deb" `
            "target/$Target/release/$LinuxBinaryName-$Version.deb" -Force
    }

} finally {
    # Delete temp directory
    Remove-Item -Path "build/temp" -Recurse -Force

    Pop-Location
}