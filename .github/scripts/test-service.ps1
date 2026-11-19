param([Parameter(Mandatory=$true)][string]$Binary)
$ErrorActionPreference = 'Stop'
function Invoke-Cli {
    & $Binary @args
    if ($LASTEXITCODE -ne 0) { throw "CLI failed: $args" }
}
# A closed local port exercises service startup without applying browser policies.
Invoke-Cli setup --url https://127.0.0.1:9/policy.yaml
Invoke-Cli install-service
Invoke-Cli start
$service = Get-Service FamilyPolicyAgent
$service.WaitForStatus('Running', [TimeSpan]::FromSeconds(20))
Invoke-Cli status
Invoke-Cli stop
$service.WaitForStatus('Stopped', [TimeSpan]::FromSeconds(20))
Invoke-Cli start
$service.WaitForStatus('Running', [TimeSpan]::FromSeconds(20))
Invoke-Cli stop
$service.WaitForStatus('Stopped', [TimeSpan]::FromSeconds(20))
Invoke-Cli uninstall-service
if (Get-Service FamilyPolicyAgent -ErrorAction SilentlyContinue) { throw 'Service was not removed' }
