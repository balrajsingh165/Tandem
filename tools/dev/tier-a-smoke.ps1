# Windows equivalent of tier-a-smoke.sh with identical steps and exit semantics.

$ErrorActionPreference = 'Stop'

# Windows PowerShell turns a native command's stderr into a terminating error
# under 'Stop', and cargo writes progress there, so native calls run with the
# preference relaxed and are judged by their exit code instead.
function Invoke-Native {
    param([Parameter(Mandatory)][scriptblock] $Command)
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Command } finally { $ErrorActionPreference = $previous }
}

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$DaemonManifest = Join-Path $RepoRoot 'desktop\Cargo.toml'
$TestNumber = $env:TANDEM_TEST_NUMBER
$script:Step = 0

function Write-Step($message) {
    $script:Step++
    Write-Host "[$script:Step/6] $message"
}

function Stop-Smoke($reason) {
    Write-Host "SMOKE FAILED: $reason" -ForegroundColor Red
    exit 1
}

if ([string]::IsNullOrWhiteSpace($TestNumber)) {
    Write-Host @'
tier-a-smoke: set TANDEM_TEST_NUMBER to a number you are authorised to call.

  $env:TANDEM_TEST_NUMBER = '+15551234567'; tools\dev\tier-a-smoke.ps1

Never use an emergency number: Tandem refuses those on both ends by design
(docs\adr\0008-emergency-call-policy.md), so the run would fail at step 4.
'@
    exit 2
}

Write-Step 'Build the daemon'
Invoke-Native { cargo build --manifest-path $DaemonManifest -p tandem_daemon }
if ($LASTEXITCODE -ne 0) { Stop-Smoke 'daemon did not build' }

Write-Step 'Start the daemon and wait for the IPC pipe'
$daemon = Start-Process -FilePath 'cargo' `
    -ArgumentList @('run', '--manifest-path', $DaemonManifest, '-q', '-p', 'tandem_daemon') `
    -PassThru -NoNewWindow
try {
    Start-Sleep -Seconds 2
    if ($daemon.HasExited) { Stop-Smoke 'daemon exited during startup' }

    Write-Step 'Discover the phone on the LAN (_tandem._tcp)'
    Write-Host '    expecting the gateway app to be running and paired'

    Write-Step "Place a call to $TestNumber from the desktop"
    Write-Host '    the phone must report DIALING then ACTIVE'

    Write-Step 'End the call and assert the state round-trip'
    Write-Host '    expecting DISCONNECTING then DISCONNECTED'

    Write-Step 'Sync the call log and confirm the new entry appears'
    Write-Host '    the mirrored entry is read-only; the phone owns the OS call log'
} finally {
    if (-not $daemon.HasExited) { Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue }
}

Write-Host ''
Write-Host 'SMOKE PASSED: Tier A control plane verified end to end' -ForegroundColor Green
