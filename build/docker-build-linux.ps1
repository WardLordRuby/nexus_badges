param(
    [string]$BinaryName = "nexus_badges",
    [string]$Target = "x86_64-unknown-linux-gnu",
    [Parameter(Mandatory = $true)]
    [string]$Version
)

# Define metadata
if ($Target -eq "x86_64-unknown-linux-gnu") {
    $Architecture = "amd64"
} elseif ($Target -eq "aarch64-unknown-linux-gnu") {
    $Architecture = "arm64"
} else {
    Write-Error "unsupported target"
    exit 1
}

# Ensure BinaryName is linux friendly
$LinuxBinaryName = $BinaryName.Replace('_', '-')

# Capture the current directory at the start
Push-Location

# Ensure script always has the same working directory
Set-Location -Path $PSScriptRoot

# Change to project root
Push-Location (Split-Path -Parent $PSScriptRoot)

# Build binary in release mode for desired target
cross build --target $Target --release
if ($LASTEXITCODE -ne 0) { exit 1 }

if (Get-Command "debforge" -ErrorAction SilentlyContinue) {
    # Run debforge if it has an installed path variable
    debforge --binary-name $BinaryName --version $Version --target $Target
} else {
    if (-not (Test-Path "build/debforge.exe")) {
        # Download debforge (automates the creation of the debian file structure)
        git clone https://github.com/WardLordRuby/debforge.git build/debforge
    
        cargo build --release --manifest-path build/debforge/Cargo.toml
        if ($LASTEXITCODE -ne 0) { exit 1 }
    
        Copy-Item "build/debforge/target/release/debforge.exe" "build/debforge.exe"
    
        # Remove debforge source files
        Remove-Item -Path "debforge" -Recurse -Force
    }
    
    # Create the expected debian file structure
    .\build\debforge.exe --binary-name $BinaryName --version $Version --target $Target
}

try {
    # Build the Docker image if it doesn't exist
    $ImageName = "deb-builder"
    if (-not (docker images -q $ImageName)) {
        Write-Host "Building Docker image..."
        docker build -t $ImageName ./build/debian/.
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

    # Revert back to the original directory
    Pop-Location
}