param(
    [string]$BinaryName = "nexus_badges",
    [string]$Target = "x86_64-unknown-linux-gnu",
    [Parameter(Mandatory = $true)]
    [string]$Version
)

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
Depends: libc6 (>= 2.28), libgcc1
Description: Shields.io badge generator for Nexus Mods
 The Nexus Mod Badges package includes various badge formats for showing mod
 statistics such as total downloads, unique downloads, and more. This package
 includes badges for different output formats including Markdown, HTML, and
 others. It is designed to be easily integrated with websites, README files,
 or any other platform supporting badges.
"@

$PostRM = @"
#!/bin/sh
set -e

case "`$1" in
    purge)
        for user in `$(getent passwd | awk -F: '`$3 >= 1000' | cut -d: -f6); do
            config_dir="`$user/.config/$LinuxBinaryName"
            if [ -d "`$config_dir" ]; then
                rm -rf "`$config_dir"
            fi
        done
        ;;
esac

exit 0
"@

# Change to project root
Push-Location (Split-Path -Parent $PSScriptRoot)

try {
    # Build binary in release mode for desired target
    cross build --target $Target --release

    # Create distribution directory
    $DistDir = "build/tmp/dist/linux/$LinuxBinaryName-$Version"
    $ChangelogDir = "$DistDir/usr/share/doc/$LinuxBinaryName"
    New-Item -ItemType Directory -Force -Path $DistDir
    New-Item -ItemType Directory -Force -Path "$DistDir/usr/local/bin"
    New-Item -ItemType Directory -Force -Path "$DistDir/usr/share/applications"
    New-Item -ItemType Directory -Force -Path "$DistDir/usr/share/icons/hicolor/256x256/apps"
    New-Item -ItemType Directory -Force -Path "$ChangelogDir"

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

    # Create postrm purge script
    $PostRM | Out-File -FilePath "$DistDir/DEBIAN/postrm" -Encoding UTF8

    # Copy changelog
    Copy-Item "build/changelog" "$ChangelogDir/changelog"

    # Build the Docker image if it doesn't exist
    $ImageName = "deb-builder"
    if (-not (docker images -q $ImageName)) {
        Write-Host "Building Docker image..."
        docker build -t $ImageName ./build/.
    }

    # Run the container
    if (docker run --rm -v "${PWD}/build/tmp:/build/tmp" $ImageName -and $?) {
        Write-Host "Done! Package created in target/$Target/release/"

        Move-Item "build/tmp/dist/linux/$LinuxBinaryName-$Version.deb" `
            "target/$Target/release/${BinaryName}_linux_${Architecture}.deb" -Force
    }

} finally {
    # Delete tmp directory
    Remove-Item -Path "build/tmp" -Recurse -Force

    Pop-Location
}