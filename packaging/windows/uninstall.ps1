# Family Policy Agent - Windows Uninstallation Script
# Run as Administrator

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "ERROR: This script must be run as Administrator" -ForegroundColor Red
    Write-Host "Right-click PowerShell and select 'Run as Administrator'" -ForegroundColor Yellow
    exit 1
}

Write-Host "Family Policy Agent - Windows Uninstallation" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host ""

# Stop service if running (placeholder for future service implementation)
Write-Host "Stopping agent..." -ForegroundColor Yellow
# TODO: Stop Windows Service when implemented

# Remove from PATH
$installDir = "$env:ProgramFiles\FamilyPolicy"
$currentPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::Machine)
if ($currentPath -like "*$installDir*") {
    $newPath = ($currentPath.Split(';') | Where-Object { $_ -ne $installDir }) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $newPath, [EnvironmentVariableTarget]::Machine)
    Write-Host "✓ Removed from system PATH" -ForegroundColor Green
}

# Remove binary
if (Test-Path $installDir) {
    Remove-Item -Path $installDir -Recurse -Force
    Write-Host "✓ Binary removed" -ForegroundColor Green
}

# Ask about configuration and data
Write-Host ""
$response = Read-Host "Remove configuration and state files? (y/N)"
if ($response -eq 'y' -or $response -eq 'Y') {
    # Remove policies first
    & family-policy --uninstall 2>$null

    # Remove directories
    $configDir = "$env:ProgramData\family-policy"
    $stateDir = "$env:ProgramData\browser-extension-policy"

    if (Test-Path $configDir) {
        Remove-Item -Path $configDir -Recurse -Force
    }
    if (Test-Path $stateDir) {
        Remove-Item -Path $stateDir -Recurse -Force
    }

    Write-Host "✓ Configuration and state files removed" -ForegroundColor Green
} else {
    Write-Host "Configuration and state files preserved in:" -ForegroundColor Yellow
    Write-Host "  - $env:ProgramData\family-policy\" -ForegroundColor Gray
    Write-Host "  - $env:ProgramData\browser-extension-policy\" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Uninstallation complete!" -ForegroundColor Green
Write-Host "Please restart your PowerShell window for PATH changes to take effect." -ForegroundColor Yellow
