# GitLab Knowledge Graph (gkg) Installation Script for Windows
# Usage: irm https://example.com/install.ps1 | iex
#    or: irm https://example.com/install.ps1 -OutFile install.ps1; .\install.ps1 -Version v1.2.3
#    or: $env:GITLAB_TOKEN="your-token"; irm https://example.com/install.ps1 | iex
# To run the already downloaded script:
# Get-Content install.ps1 -Raw | iex

param(
    [string]$Version = "",
    [switch]$Force,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

# Configuration
$GkgName = "GitLab Knowledge Graph (gkg)"
$BinaryName = "gkg"
$InstallDir = "$env:LOCALAPPDATA\Programs\gkg"
$Platform = "windows"
$Arch = "x86_64"
$ProjectPath = "69095239"

# Colors for output
function Write-Success { Write-Host $args -ForegroundColor Green }
function Write-Error { Write-Host $args -ForegroundColor Red }
function Write-Warning { Write-Host $args -ForegroundColor Yellow }

# Show usage
if ($Help) {
    @"
Usage: .\install.ps1 [OPTIONS]

OPTIONS:
    -Version VERSION    Install specific version (e.g., v1.2.3)
    -Force             Force installation even if gkg already exists
    -Help              Show this help message

ENVIRONMENT VARIABLES:
    GITLAB_TOKEN       GitLab personal access token for authentication

EXAMPLES:
    # Install latest version
    irm https://example.com/install.ps1 | iex

    # Install specific version
    irm https://example.com/install.ps1 -OutFile install.ps1; .\install.ps1 -Version v1.2.3

    # Install with GitLab authentication
    `$env:GITLAB_TOKEN="your-token"; irm https://example.com/install.ps1 | iex

    # Force reinstall
    irm https://example.com/install.ps1 -OutFile install.ps1; .\install.ps1 -Force
"@
    return
}

Write-Host "=== $GkgName Installation Script ===" -ForegroundColor Cyan
Write-Host ""

# Check if curl.exe is available
$curlPath = "curl.exe"
try {
    $null = Get-Command $curlPath -ErrorAction Stop
} catch {
    Write-Error "curl.exe not found. This script requires curl.exe (available in Windows 10+)."
    return
}

# Check if gkg already exists
$gkgPath = Join-Path $InstallDir "$BinaryName.exe"
if ((Test-Path $gkgPath) -and -not $Force) {
    Write-Warning "$GkgName is already installed at $gkgPath"
    Write-Host "Use -Force to reinstall."
    return
}

# Create install directory
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# Construct download URLs using GitLab API v4
$artifactName = "$BinaryName-$Platform-$Arch.tar.gz"
if ($Version) {
    Write-Host "Installing $GkgName version $Version..."
    $downloadUrl = "https://gitlab.com/api/v4/projects/$ProjectPath/releases/$Version/downloads/$artifactName"
    $checksumUrl = "https://gitlab.com/api/v4/projects/$ProjectPath/releases/$Version/downloads/$artifactName.sha256"
} else {
    Write-Host "Installing latest version of $GkgName..."
    $downloadUrl = "https://gitlab.com/api/v4/projects/$ProjectPath/releases/permalink/latest/downloads/$artifactName"
    $checksumUrl = "https://gitlab.com/api/v4/projects/$ProjectPath/releases/permalink/latest/downloads/$artifactName.sha256"
}

# Prepare curl arguments
$curlArgs = @(
    "--fail",           # Fail on HTTP errors
    "--location",       # Follow redirects
    "--silent",         # Silent mode
    "--show-error"      # Show errors
)

if ($env:GITLAB_TOKEN) {
    Write-Host "Using GitLab authentication token..."
    $curlArgs += @("--header", "Authorization: Bearer $env:GITLAB_TOKEN")
}

# Download the tarball
$tempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
$tarballPath = Join-Path $tempDir $artifactName

try {
    Write-Host "Downloading $GkgName for $Platform-$Arch..."
    $downloadArgs = $curlArgs + @("--output", $tarballPath, $downloadUrl)
    
    $result = & $curlPath @downloadArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "curl failed with exit code $LASTEXITCODE. Output: $result"
    }
} catch {
    if (-not $env:GITLAB_TOKEN) {
        Write-Error "Failed to download $GkgName. If the repository requires authentication, please set GITLAB_TOKEN environment variable."
    } else {
        Write-Error "Failed to download $GkgName. Please check your internet connection, GitLab token permissions, and the version number."
    }
    Write-Error "Error details: $_"
    Remove-Item -Recurse -Force $tempDir
    return
}

# Download and verify checksum
Write-Host "Downloading checksum..."
$checksumPath = Join-Path $tempDir "$artifactName.sha256"
try {
    $checksumArgs = $curlArgs + @("--output", $checksumPath, $checksumUrl)
    
    $result = & $curlPath @checksumArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "curl failed with exit code $LASTEXITCODE. Output: $result"
    }
} catch {
    Write-Error "Failed to download checksum file. Cannot verify download integrity."
    Write-Error "Error details: $_"
    Remove-Item -Recurse -Force $tempDir
    return
}

Write-Host "Verifying checksum..."
try {
    $expectedChecksum = (Get-Content $checksumPath -Raw).Trim().Split()[0]
    $actualChecksum = (Get-FileHash $tarballPath -Algorithm SHA256).Hash.ToLower()
    
    if ($expectedChecksum -ne $actualChecksum) {
        throw "Checksum verification failed!`nExpected: $expectedChecksum`nActual: $actualChecksum"
    }
    Write-Success "Checksum verified successfully."
} catch {
    Write-Error $_
    Remove-Item -Recurse -Force $tempDir
    return
}

# Extract the tarball (requires tar in Windows 10+)
Write-Host "Extracting $BinaryName..."
try {
    tar -xzf $tarballPath -C $tempDir
} catch {
    Write-Error "Failed to extract the tarball. Make sure 'tar' is available (Windows 10+ required)."
    Remove-Item -Recurse -Force $tempDir
    return
}

# Find all files to install (executable and DLLs)
$extractedFiles = Get-ChildItem -Path $tempDir -Recurse -File | Where-Object { 
    $_.Extension -eq ".exe" -or $_.Extension -eq ".dll" 
}

# Ensure we have the main binary
$mainBinary = $extractedFiles | Where-Object { 
    $_.Name -eq "$BinaryName.exe" -or $_.Name -eq $BinaryName 
} | Select-Object -First 1

if (-not $mainBinary) {
    Write-Error "$BinaryName binary not found in the extracted files."
    Remove-Item -Recurse -Force $tempDir
    return
}

# Install all files
Write-Host "Installing $BinaryName and dependencies to $InstallDir..."
$installedFiles = @()
foreach ($file in $extractedFiles) {
    $destPath = Join-Path $InstallDir $file.Name
    Move-Item -Path $file.FullName -Destination $destPath -Force
    $installedFiles += $file.Name
}

Write-Host "Installed files: $($installedFiles -join ', ')"
$finalPath = Join-Path $InstallDir $mainBinary.Name

# Clean up temp directory
Remove-Item -Recurse -Force $tempDir

# Update PATH
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$InstallDir*") {
    Write-Host "Updating PATH..."
    $newPath = "$InstallDir;$userPath"
    [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    
    # Update current session
    $env:PATH = "$InstallDir;$env:PATH"
    
    Write-Warning "PATH has been updated. You may need to restart your terminal for changes to take effect."
} else {
    Write-Host "PATH already contains $InstallDir"
}

Write-Success "$GkgName has been successfully installed to $finalPath"
Write-Host ""
Write-Success "Installation complete!"
Write-Host ""
Write-Host "To verify the installation, run: gkg --version"
Write-Host ""
Write-Host "If the command is not found, please restart your terminal or run:"
Write-Host "  `$env:PATH = `"$InstallDir;`$env:PATH`""
