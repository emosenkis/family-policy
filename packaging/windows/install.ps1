# Family Policy Agent - Windows Installation Script
# Run as Administrator

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "ERROR: This script must be run as Administrator" -ForegroundColor Red
    Write-Host "Right-click PowerShell and select 'Run as Administrator'" -ForegroundColor Yellow
    exit 1
}

Write-Host "Family Policy Agent - Windows Installation" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# Install binary
Write-Host "Installing binary..." -ForegroundColor Yellow
$binaryPath = "$env:ProgramFiles\FamilyPolicy\family-policy.exe"
$installDir = "$env:ProgramFiles\FamilyPolicy"

if (-not (Test-Path ".\family-policy.exe")) {
    Write-Host "ERROR: family-policy.exe not found in current directory" -ForegroundColor Red
    Write-Host "Please run this script from the directory containing the binary" -ForegroundColor Yellow
    exit 1
}

# Create install directory
New-Item -ItemType Directory -Force -Path $installDir | Out-Null

# Copy binary
Copy-Item ".\family-policy.exe" -Destination $binaryPath -Force
Write-Host "✓ Binary installed to $binaryPath" -ForegroundColor Green

# Add to PATH
$currentPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::Machine)
if ($currentPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$currentPath;$installDir", [EnvironmentVariableTarget]::Machine)
    Write-Host "✓ Added to system PATH" -ForegroundColor Green
}

# Create necessary directories
Write-Host "Creating directories..." -ForegroundColor Yellow
$configDir = "$env:ProgramData\family-policy"
$stateDir = "$env:ProgramData\browser-extension-policy"

New-Item -ItemType Directory -Force -Path $configDir | Out-Null
New-Item -ItemType Directory -Force -Path $stateDir | Out-Null
Write-Host "✓ Directories created" -ForegroundColor Green

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Open a new PowerShell window (to refresh PATH)" -ForegroundColor White
Write-Host ""
Write-Host "  2. Configure the agent:" -ForegroundColor White
Write-Host "     family-policy config init" -ForegroundColor Gray
Write-Host "     # Edit the generated config file, then apply it:" -ForegroundColor Gray
Write-Host "     family-policy --config family-policy.yaml" -ForegroundColor Gray
Write-Host ""
Write-Host "  3. Install as Windows Service (optional, recommended):" -ForegroundColor White
Write-Host "     family-policy install-service" -ForegroundColor Gray
Write-Host ""
Write-Host "  4. Start the agent:" -ForegroundColor White
Write-Host "     family-policy start              # Start as service" -ForegroundColor Gray
Write-Host "     family-policy start --no-daemon  # Or run in foreground" -ForegroundColor Gray
Write-Host ""
Write-Host "  5. Check status:" -ForegroundColor White
Write-Host "     family-policy status" -ForegroundColor Gray
Write-Host ""
