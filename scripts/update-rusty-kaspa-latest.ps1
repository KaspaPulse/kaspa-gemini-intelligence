param(
    [string]$KaspaRepoApi = "https://api.github.com/repos/kaspanet/rusty-kaspa",
    [string]$KaspaGitUrl = "https://github.com/kaspanet/rusty-kaspa.git",
    [switch]$AllowPrerelease,
    [switch]$Push,
    [switch]$NoBranch,
    [string]$BaseBranch = "dev"
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
Set-StrictMode -Version Latest

function Step($Name, [scriptblock]$Block) {
    Write-Host "`n==> $Name" -ForegroundColor Cyan
    & $Block
    if ($LASTEXITCODE -ne $null -and $LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
    Write-Host "OK: $Name" -ForegroundColor Green
}

function Run-AllowFail($Name, [scriptblock]$Block) {
    Write-Host "`n==> $Name" -ForegroundColor Cyan
    & $Block
    $code = $LASTEXITCODE
    if ($code -ne $null -and $code -ne 0) {
        Write-Host "FAILED: $Name exit code $code" -ForegroundColor Red
        return $false
    }
    Write-Host "OK: $Name" -ForegroundColor Green
    return $true
}

function Get-LatestKaspaReleaseTag {
    param([switch]$AllowPrerelease)

    $headers = @{
        "Accept" = "application/vnd.github+json"
        "User-Agent" = "KaspaPulse-AutoUpdater"
    }

    try {
        if ($AllowPrerelease) {
            $releases = Invoke-RestMethod -Uri "$KaspaRepoApi/releases?per_page=50" -Headers $headers
            $release = $releases |
                Where-Object { -not $_.draft } |
                Sort-Object { [datetime]$_.published_at } -Descending |
                Select-Object -First 1

            if ($null -ne $release -and -not [string]::IsNullOrWhiteSpace($release.tag_name)) {
                return [string]$release.tag_name
            }
        } else {
            $release = Invoke-RestMethod -Uri "$KaspaRepoApi/releases/latest" -Headers $headers
            if ($null -ne $release -and -not [string]::IsNullOrWhiteSpace($release.tag_name)) {
                return [string]$release.tag_name
            }
        }
    }
    catch {
        Write-Host "GitHub release API failed, falling back to git tags: $($_.Exception.Message)" -ForegroundColor Yellow
    }

    $tagsRaw = git ls-remote --tags --refs $KaspaGitUrl "refs/tags/v*"
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to list rusty-kaspa tags"
    }

    $tags = @()

    foreach ($line in $tagsRaw) {
        if ($line -match "refs/tags/(v\d+\.\d+\.\d+)$") {
            $tag = $Matches[1]
            if ($tag -match "^v(\d+)\.(\d+)\.(\d+)$") {
                $tags += [pscustomobject]@{
                    Tag = $tag
                    Major = [int]$Matches[1]
                    Minor = [int]$Matches[2]
                    Patch = [int]$Matches[3]
                }
            }
        }
    }

    if ($tags.Count -eq 0) {
        throw "No stable semver rusty-kaspa tags found."
    }

    return ($tags | Sort-Object Major, Minor, Patch -Descending | Select-Object -First 1).Tag
}

function Get-CurrentKaspaTags {
    $cargo = Get-Content "Cargo.toml" -Raw -Encoding UTF8
    $matches = [regex]::Matches(
        $cargo,
        'git\s*=\s*"https://github\.com/kaspanet/rusty-kaspa"\s*,\s*tag\s*=\s*"([^"]+)"'
    )

    $tags = @()
    foreach ($m in $matches) {
        $tags += $m.Groups[1].Value
    }

    return $tags | Sort-Object -Unique
}

function Update-KaspaTags {
    param([string]$TargetTag)

    $path = "Cargo.toml"
    $content = Get-Content $path -Raw -Encoding UTF8

    $new = [regex]::Replace(
        $content,
        '(git\s*=\s*"https://github\.com/kaspanet/rusty-kaspa"\s*,\s*tag\s*=\s*")[^"]+(")',
        ('$1' + $TargetTag + '$2')
    )

    if ($new -eq $content) {
        throw "No rusty-kaspa git tag entries were changed in Cargo.toml."
    }

    Set-Content $path $new -Encoding UTF8
}

Step "Verify working tree" {
    git status --short --branch
}

Step "Find latest rusty-kaspa release tag automatically" {
    $script:LatestTag = Get-LatestKaspaReleaseTag -AllowPrerelease:$AllowPrerelease
    Write-Host "Latest rusty-kaspa tag: $script:LatestTag"

    $script:CurrentTags = @(Get-CurrentKaspaTags)
    Write-Host "Current rusty-kaspa tags in Cargo.toml: $($script:CurrentTags -join ', ')"

    if ($script:CurrentTags.Count -eq 0) {
        throw "No rusty-kaspa tag dependencies found in Cargo.toml."
    }
}

if ($CurrentTags.Count -eq 1 -and $CurrentTags[0] -eq $LatestTag) {
    Write-Host "`nAlready on latest rusty-kaspa tag: $LatestTag" -ForegroundColor Green
    exit 0
}

if (-not $NoBranch) {
    Step "Create auto update branch" {
        $safeTag = $LatestTag -replace '[^a-zA-Z0-9_.-]', '-'
        $branch = "auto/rusty-kaspa-$safeTag"

        git fetch origin
        git checkout $BaseBranch

        $remoteRef = "origin/$BaseBranch"
        $existsRemote = git rev-parse --verify $remoteRef 2>$null
        if ($LASTEXITCODE -eq 0) {
            git reset --hard $remoteRef
        }

        $existing = git branch --list $branch
        if ($existing) {
            git branch -D $branch
        }

        git checkout -b $branch
        $script:UpdateBranch = $branch
        Write-Host "Update branch: $branch"
    }
} else {
    $script:UpdateBranch = (git branch --show-current).Trim()
}

Step "Update Cargo.toml rusty-kaspa tags" {
    Update-KaspaTags -TargetTag $LatestTag
    Select-String -Path "Cargo.toml" -Pattern "kaspanet/rusty-kaspa|tag ="
}

Step "Update Cargo.lock" {
    cargo update
}

$env:SQLX_OFFLINE = "true"
$env:CARGO_INCREMENTAL = "0"
$env:RUST_BACKTRACE = "1"

$AllGood = $true

if (-not (Run-AllowFail "cargo fmt" {
    cargo fmt --all
})) { $AllGood = $false }

if (-not (Run-AllowFail "cargo check" {
    cargo check --locked --all-targets --all-features
})) { $AllGood = $false }

if (-not (Run-AllowFail "cargo clippy" {
    cargo clippy --locked --all-targets --all-features -- -D warnings
})) { $AllGood = $false }

if (-not (Run-AllowFail "cargo test" {
    cargo test --locked --all-targets --all-features
})) { $AllGood = $false }

if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
    if (-not (Run-AllowFail "cargo audit" {
        cargo audit
    })) { $AllGood = $false }
} else {
    Write-Host "cargo-audit not installed; skipping." -ForegroundColor Yellow
    $AllGood = $false
}

if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
    if (-not (Run-AllowFail "cargo deny check" {
        cargo deny check
    })) { $AllGood = $false }
} else {
    Write-Host "cargo-deny not installed; skipping." -ForegroundColor Yellow
    $AllGood = $false
}

if (Test-Path "scripts\security-check.ps1") {
    if (-not (Run-AllowFail "project security-check.ps1" {
        powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\security-check.ps1"
    })) { $AllGood = $false }
}

Step "Show final diff" {
    git status --short --branch
    git diff --stat
}

if (-not $AllGood) {
    Write-Host "`nAutomatic Kaspa update failed validation. Do not merge or deploy." -ForegroundColor Red
    exit 1
}

if ($Push) {
    Step "Push auto update branch" {
        git push origin $UpdateBranch --force-with-lease
    }
}

Write-Host "`nAutomatic Kaspa update passed." -ForegroundColor Green
Write-Host "Branch: $UpdateBranch"
Write-Host "LatestTag: $LatestTag"
exit 0
