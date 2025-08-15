param(
    [string]$KuzuVersion = $null
)

function Get-KuzuVersionFromCargo {
    Write-Host "Reading Kuzu version from Cargo.toml..."
    
    if (Test-Path "Cargo.toml") {
        $cargoContent = Get-Content "Cargo.toml" -Raw
        $kuzuMatch = [regex]::Match($cargoContent, 'kuzu\s*=\s*"([^"]+)"')
        if ($kuzuMatch.Success) {
            $version = $kuzuMatch.Groups[1].Value
            Write-Host "Found Kuzu version in Cargo.toml: $version"
            return $version
        }
    }
    
    throw "Could not find Kuzu version in Cargo.toml"
}

New-Item -ItemType Directory -Force -Path C:\install\libkuzu

Write-Host "Downloading rustup bootstrapper..."
curl.exe -L -o C:\install\rustup-init.exe https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe

Write-Host "Installing Rust toolchain..."
& C:\install\rustup-init.exe -y --default-toolchain 1.88.0 --default-host x86_64-pc-windows-msvc | Write-Host
$env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH","User")

Write-Host "Installing LLVM for ruby-prism build..."

choco install llvm

Write-Host "All installations complete."

if ([string]::IsNullOrEmpty($KuzuVersion)) {
    Write-Host "No Kuzu version specified, reading from Cargo.toml..."
    $KuzuVersion = Get-KuzuVersionFromCargo
} else {
    Write-Host "Using specified Kuzu version: $KuzuVersion"
}

# Clean version string (remove any version operators like ^, ~, etc.)
$CleanVersion = $KuzuVersion -replace '[^\d\.]', ''
Write-Host "Using Kuzu version: $CleanVersion"

# Download libkuzu binaries using dynamic version
Write-Host "Downloading libkuzu version $CleanVersion..."
$kuzuUrl = "https://github.com/kuzudb/kuzu/releases/download/v$CleanVersion/libkuzu-windows-x86_64.zip"
Write-Host "Download URL: $kuzuUrl"
curl.exe -L -o C:\install\libkuzu-windows-x86_64.zip $kuzuUrl

# Extract Kuzu dynamic library files
Write-Host "Extracting libkuzu..."
Expand-Archive -Path C:\install\libkuzu-windows-x86_64.zip -DestinationPath C:\install\libkuzu
Write-Host "Extraction complete. Files:"
dir C:\install\libkuzu\

# Set environment variables
$env:KUZU_SHARED = 1
$env:KUZU_INCLUDE_DIR = "C:\install\libkuzu"
$env:KUZU_LIBRARY_DIR = "C:\install\libkuzu"

Write-Host "Kuzu installation complete. Kuzu version: $CleanVersion"
