# Windows equivalent of gen-proto.sh: verifies protoc availability and triggers
# Kotlin and Rust generation from /proto.

$ErrorActionPreference = 'Stop'

# Windows PowerShell turns a native command's stderr into a terminating error
# under 'Stop', and cargo writes progress there, so native calls run with the
# preference relaxed and are judged by their exit code instead.
function Invoke-Native {
    param([Parameter(Mandatory)][scriptblock] $Command, [string] $What = 'command')
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Command } finally { $ErrorActionPreference = $previous }
    if ($LASTEXITCODE -ne 0) { Write-Error "$What failed with exit code $LASTEXITCODE" }
}

$RepoRoot = Split-Path -Parent $PSScriptRoot
$ProtoDir = Join-Path $RepoRoot 'proto'

if (-not (Test-Path $ProtoDir)) {
    Write-Error "gen-proto: no proto directory at $ProtoDir"
}

Write-Host '==> Schema files'
Get-ChildItem -Path $ProtoDir -Filter *.proto -Recurse | Sort-Object FullName | ForEach-Object {
    Write-Host "    $($_.FullName)"
}

# The Rust build vendors its own protoc, so a system protoc is optional and used
# only for the standalone syntax check below.
$protoc = Get-Command protoc -ErrorAction SilentlyContinue
if ($protoc) {
    Write-Host "==> protoc syntax check ($(& protoc --version))"
    $protoFiles = Get-ChildItem -Path (Join-Path $ProtoDir 'tandem\v1') -Filter *.proto |
        ForEach-Object { $_.FullName }
    Invoke-Native { & protoc --proto_path=$ProtoDir -o NUL @protoFiles } 'protoc syntax check'
} else {
    Write-Host '==> protoc not found; skipping standalone check (Rust codegen vendors it)'
}

Write-Host '==> Rust bindings (tandem_proto)'
$desktopManifest = Join-Path $RepoRoot 'desktop\Cargo.toml'
Invoke-Native { cargo build --manifest-path $desktopManifest -p tandem_proto } 'cargo build'

$gradlew = Join-Path $RepoRoot 'android\gradlew.bat'
if (Test-Path $gradlew) {
    Write-Host '==> Kotlin bindings (:app:generateProto)'
    Push-Location (Join-Path $RepoRoot 'android')
    try {
        Invoke-Native { & $gradlew --quiet :app:generateDebugProto } 'gradle codegen'
    } finally { Pop-Location }
} else {
    Write-Host '==> android\gradlew.bat not present; skipping Kotlin codegen'
}

Write-Host '==> Protocol bindings up to date'
