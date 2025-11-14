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

# Note about Windows Service
Write-Host ""
Write-Host "NOTE: Windows Service installation requires the service wrapper." -ForegroundColor Yellow
Write-Host "For now, you can run the agent manually or use Task Scheduler." -ForegroundColor Yellow

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Open a new PowerShell window (to refresh PATH)" -ForegroundColor White
Write-Host ""
Write-Host "  2. Configure the agent:" -ForegroundColor White
Write-Host "     family-policy agent setup ``" -ForegroundColor Gray
Write-Host "       --url https://raw.githubusercontent.com/USER/REPO/main/policy.yaml ``" -ForegroundColor Gray
Write-Host "       --token YOUR_GITHUB_TOKEN" -ForegroundColor Gray
Write-Host ""
Write-Host "  3. Start the agent:" -ForegroundColor White
Write-Host "     family-policy agent start --no-daemon" -ForegroundColor Gray
Write-Host ""
Write-Host "  4. Check status:" -ForegroundColor White
Write-Host "     family-policy agent status" -ForegroundColor Gray
Write-Host ""
