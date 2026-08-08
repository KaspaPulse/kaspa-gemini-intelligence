param(
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
    $global:LASTEXITCODE = 0
    & $Block
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
    Write-Host "OK: $Name" -ForegroundColor Green
}

function Run-AllowFail($Name, [scriptblock]$Block) {
    Write-Host "`n==> $Name" -ForegroundColor Cyan
    $global:LASTEXITCODE = 0
    & $Block
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        Write-Host "FAILED: $Name exit code $code" -ForegroundColor Red
        return $false
    }
    Write-Host "OK: $Name" -ForegroundColor Green
    return $true
}

function ConvertTo-KaspaSemver {
    param([Parameter(Mandatory = $true)][string]$Tag)

    if ($Tag -notmatch '^v(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$') {
        return $null
    }

    $prerelease = $Matches[4]
    return [pscustomobject]@{
        Tag = $Tag
        Major = [int]$Matches[1]
        Minor = [int]$Matches[2]
        Patch = [int]$Matches[3]
        Prerelease = $prerelease
        StableRank = if ([string]::IsNullOrWhiteSpace($prerelease)) { 1 } else { 0 }
    }
}

function Compare-KaspaSemverCore {
    param(
        [Parameter(Mandatory = $true)]$Left,
        [Parameter(Mandatory = $true)]$Right
    )

    foreach ($property in @('Major', 'Minor', 'Patch')) {
        if ($Left.$property -gt $Right.$property) { return 1 }
        if ($Left.$property -lt $Right.$property) { return -1 }
    }

    if ($Left.StableRank -gt $Right.StableRank) { return 1 }
    if ($Left.StableRank -lt $Right.StableRank) { return -1 }
    return 0
}

function Get-LatestKaspaReleaseTag {
    param([switch]$AllowPrerelease)

    $tagsRaw = git ls-remote --tags --refs $KaspaGitUrl "refs/tags/v*"
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to list rusty-kaspa tags"
    }

    $tags = @()
    foreach ($line in $tagsRaw) {
        if ($line -notmatch 'refs/tags/(v[^\s]+)$') {
            continue
        }

        $candidate = ConvertTo-KaspaSemver -Tag $Matches[1]
        if ($null -eq $candidate) {
            continue
        }
        if (-not $AllowPrerelease -and $candidate.StableRank -eq 0) {
            continue
        }
        $tags += $candidate
    }

    if ($tags.Count -eq 0) {
        $kind = if ($AllowPrerelease) { "semver" } else { "stable semver" }
        throw "No $kind rusty-kaspa tags found."
    }

    $latest = $tags |
        Sort-Object `
            @{ Expression = 'Major'; Descending = $true },
            @{ Expression = 'Minor'; Descending = $true },
            @{ Expression = 'Patch'; Descending = $true },
            @{ Expression = 'StableRank'; Descending = $true },
            @{ Expression = 'Prerelease'; Descending = $true } |
        Select-Object -First 1

    return $latest.Tag
}

function Get-CurrentKaspaTags {
    $cargo = Get-Content "Cargo.toml" -Raw -Encoding UTF8
    $matches = [regex]::Matches(
        $cargo,
        'git\s*=\s*"https://github\.com/kaspanet/rusty-kaspa"\s*,\s*tag\s*=\s*"([^"]+)"'
    )

    $tags = @()
    foreach ($match in $matches) {
        $tags += $match.Groups[1].Value
    }

    return $tags | Sort-Object -Unique
}

function Update-KaspaTags {
    param([Parameter(Mandatory = $true)][string]$TargetTag)

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

Step "Verify clean working tree" {
    git status --short --branch
    $dirty = @(git status --porcelain)
    if ($dirty.Count -ne 0) {
        throw "Working tree is not clean. Refusing automated dependency mutation."
    }
}

Step "Find highest rusty-kaspa semver tag" {
    $script:LatestTag = Get-LatestKaspaReleaseTag -AllowPrerelease:$AllowPrerelease
    Write-Host "Highest eligible rusty-kaspa tag: $script:LatestTag"

    $script:CurrentTags = @(Get-CurrentKaspaTags)
    Write-Host "Current rusty-kaspa tags in Cargo.toml: $($script:CurrentTags -join ', ')"

    if ($script:CurrentTags.Count -eq 0) {
        throw "No rusty-kaspa tag dependencies found in Cargo.toml."
    }
    if ($script:CurrentTags.Count -ne 1) {
        throw "rusty-kaspa dependencies are not pinned to one consistent tag."
    }

    $currentVersion = ConvertTo-KaspaSemver -Tag $script:CurrentTags[0]
    $latestVersion = ConvertTo-KaspaSemver -Tag $script:LatestTag
    if ($null -eq $currentVersion -or $null -eq $latestVersion) {
        throw "Unable to compare current and target rusty-kaspa tags safely."
    }

    if ((Compare-KaspaSemverCore -Left $currentVersion -Right $latestVersion) -gt 0) {
        throw "Refusing downgrade from $($script:CurrentTags[0]) to $script:LatestTag."
    }
}

if ($script:CurrentTags[0] -eq $script:LatestTag) {
    Write-Host "`nAlready on highest eligible rusty-kaspa tag: $script:LatestTag" -ForegroundColor Green
    exit 0
}

if (-not $NoBranch) {
    Step "Create update branch from checked-out base" {
        $currentBranch = (git branch --show-current).Trim()
        if ($currentBranch -ne $BaseBranch) {
            throw "Expected checked-out base branch '$BaseBranch' but found '$currentBranch'."
        }

        $safeTag = $script:LatestTag -replace '[^a-zA-Z0-9_.-]', '-'
        $branch = "auto/rusty-kaspa-$safeTag"
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
    if ([string]::IsNullOrWhiteSpace($script:UpdateBranch)) {
        throw "No current branch is available for -NoBranch mode."
    }
}

Step "Update Cargo.toml rusty-kaspa tags" {
    Update-KaspaTags -TargetTag $script:LatestTag
    Select-String -Path "Cargo.toml" -Pattern "kaspanet/rusty-kaspa|tag ="
}

Step "Refresh Cargo.lock" {
    cargo update
}

$env:SQLX_OFFLINE = "true"
$env:CARGO_INCREMENTAL = "0"
$env:RUST_BACKTRACE = "1"
$allGood = $true

if (-not (Run-AllowFail "cargo fmt" { cargo fmt --all })) { $allGood = $false }
if (-not (Run-AllowFail "cargo check" { cargo check --locked --all-targets --all-features })) { $allGood = $false }
if (-not (Run-AllowFail "cargo clippy" { cargo clippy --locked --all-targets --all-features -- -D warnings })) { $allGood = $false }
if (-not (Run-AllowFail "cargo test" { cargo test --locked --all-targets --all-features })) { $allGood = $false }

if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
    if (-not (Run-AllowFail "cargo audit" { cargo audit })) { $allGood = $false }
} else {
    Write-Host "cargo-audit not installed; validation is incomplete." -ForegroundColor Yellow
    $allGood = $false
}

if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
    if (-not (Run-AllowFail "cargo deny check" { cargo deny check })) { $allGood = $false }
} else {
    Write-Host "cargo-deny not installed; validation is incomplete." -ForegroundColor Yellow
    $allGood = $false
}

if (Test-Path "scripts/security-check.ps1") {
    if (-not (Run-AllowFail "project security-check.ps1" {
        pwsh -NoProfile -File "scripts/security-check.ps1"
    })) { $allGood = $false }
}

Step "Show final diff" {
    git status --short --branch
    git diff --check
    git diff --stat
}

if (-not $allGood) {
    Write-Host "`nAutomatic Kaspa update failed validation. Do not merge or deploy." -ForegroundColor Red
    exit 1
}

if ($Push) {
    Step "Commit validated update" {
        git add Cargo.toml Cargo.lock
        $pending = @(git diff --cached --name-only)
        if ($pending.Count -eq 0) {
            throw "Validation passed but no update changes are staged."
        }
        git commit -m "chore(deps): update rusty-kaspa to $script:LatestTag"
    }

    Step "Push validated update branch" {
        git push --set-upstream origin $script:UpdateBranch --force-with-lease
    }
}

Write-Host "`nAutomatic Kaspa update passed." -ForegroundColor Green
Write-Host "Branch: $script:UpdateBranch"
Write-Host "LatestTag: $script:LatestTag"
exit 0
