$ErrorActionPreference = "Stop"

Write-Host "Kaspa Pulse - Security Check Pipeline" -ForegroundColor Cyan

if (!(Test-Path "Cargo.toml")) {
    throw "Run this script from the Rust project root folder."
}

function Run-Native {
    param(
        [string]$Name,
        [string]$Exe,
        [string[]]$ArgsList
    )

    Write-Host ""
    Write-Host "==> $Name" -ForegroundColor Cyan

    & $Exe @ArgsList
    $exitCode = $LASTEXITCODE

    if ($null -ne $exitCode -and $exitCode -ne 0) {
        throw "$Name failed with exit code $exitCode"
    }

    Write-Host "OK: $Name" -ForegroundColor Green
}

Run-Native "cargo audit" "cargo" @("audit")

Run-Native "cargo deny check" "cargo" @("deny", "check")

Run-Native "cargo tree duplicate dependency scan" "cargo" @("tree", "-d")

Run-Native "cargo machete unused dependency scan" "cargo" @("machete")

Run-Native "cargo clippy strict warnings" "cargo" @(
    "clippy",
    "--all-targets",
    "--all-features",
    "--",
    "-D",
    "warnings"
)

Run-Native "cargo test" "cargo" @("test")

Write-Host ""
Write-Host "All supply-chain and security checks completed successfully." -ForegroundColor Green
