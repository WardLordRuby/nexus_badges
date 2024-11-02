param(
    [string]$BinaryName = "nexus_badges",
    [string]$Target = "x86_64-unknown-linux-gnu",    
    [string]$Version = "1.0.0"
)

# Get project root directory (assuming script is in scripts folder)
$ProjectRoot = Split-Path -Parent $PSScriptRoot

# Change to project root
Push-Location $ProjectRoot

try {
    cross build --target $Target --release

    # Set Architecture var
    if ($Target -eq "x86_64-unknown-linux-gnu") {
        $Architecture = "amd64"
    } elseif ($Target -eq "aarch64-unknown-linux-gnu") {
        $Architecture = "arm64"
    } else {
        Write-Error "unsupported target"
        exit 1
    }

    # Create distribution directory
    $LinuxBinaryName = $BinaryName.Replace('_', '-')
    $DistDir = "build/temp/dist/linux/$LinuxBinaryName-$Version"
    New-Item -ItemType Directory -Force -Path $DistDir
    New-Item -ItemType Directory -Force -Path "$DistDir/usr/local/bin"
    New-Item -ItemType Directory -Force -Path "$DistDir/usr/share/applications"
    New-Item -ItemType Directory -Force -Path "$DistDir/usr/share/icons/hicolor/256x256/apps"

    # Copy binary
    Copy-Item "target/$Target/release/$BinaryName" `
        "$DistDir/usr/local/bin/$LinuxBinaryName"

    # Copy icon
    Copy-Item "assets/Icon.png" `
        "$DistDir/usr/share/icons/hicolor/256x256/apps/$LinuxBinaryName.png"

    # Create .desktop file
@"
[Desktop Entry]
Version=$Version
Type=Application
Name=$LinuxBinaryName
Comment=Your application description
Exec=/usr/local/bin/$LinuxBinaryName
Icon=$LinuxBinaryName
Categories=Utility;
Terminal=true
"@ | `
    Out-File -FilePath "$DistDir/usr/share/applications/$LinuxBinaryName.desktop" -Encoding UTF8

    # Create control file for debian package
    New-Item -ItemType Directory -Force -Path "$DistDir/DEBIAN"

@"
Package: $LinuxBinaryName
Version: $Version
Section: utils
Priority: optional
Architecture: $Architecture
Maintainer: WardLordRuby
Description: Shields.io badge generator for Nexus Mods
"@ | `
    Out-File -FilePath "$DistDir/DEBIAN/control" -Encoding UTF8

    # Build the Docker image if it doesn't exist
    $ImageName = "deb-builder"
    if (-not (docker images -q $ImageName)) {
        Write-Host "Building Docker image..."
        docker build -t $ImageName ./build/.
    }

    # Run the container
    if (docker run --rm -v "${PWD}/build/temp:/build/temp" $ImageName -and $?) {
        # Command succeeded, do something here
        Write-Host "Done! Package created in dist/linux/"
    }

    Move-Item -Path "build/temp/dist/linux/$LinuxBinaryName-$Version.deb" -Destination "target/$Target/release/$LinuxBinaryName-$Version.deb"

} finally {
    # Force delete (don't ask for confirmation, ignore read-only)
    Remove-Item -Path "build/temp" -Recurse -Force

    Pop-Location
}