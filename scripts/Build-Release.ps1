param([switch]$Offline, [switch]$PackageOnly)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repositoryPath = (Resolve-Path -LiteralPath (Split-Path -Parent $PSScriptRoot)).Path
Push-Location -LiteralPath $repositoryPath
try {
    $hostInfo = & rustc -vV
    if ($LASTEXITCODE -ne 0 -or $hostInfo -notcontains 'host: x86_64-pc-windows-msvc') { throw 'Build on Windows x64 with the MSVC Rust toolchain.' }
    $cargoText = Get-Content -LiteralPath (Join-Path $repositoryPath 'Cargo.toml') -Raw
    $versionMatch = [regex]::Match($cargoText, '(?m)^version = "([^"]+)"')
    if (-not $versionMatch.Success) { throw 'Package version is missing.' }
    $version = $versionMatch.Groups[1].Value
    $networkArgs = @()
    if ($Offline) { $networkArgs = @('--offline') }
    if (-not $PackageOnly) {
        & cargo fmt --all -- --check
        if ($LASTEXITCODE -ne 0) { throw 'Formatting failed.' }
        & cargo clippy --release --locked @networkArgs --all-targets --jobs 2 -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw 'Clippy failed.' }
        & cargo test --release --locked @networkArgs --lib --tests --jobs 2
        if ($LASTEXITCODE -ne 0) { throw 'Tests failed.' }
        & python (Join-Path $PSScriptRoot 'generate_notices.py')
        if ($LASTEXITCODE -ne 0) { throw 'License notice generation failed.' }
        & cargo build --release --locked @networkArgs --jobs 2
        if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    }
    $source = Join-Path $repositoryPath 'target\release\CutPlanner.exe'
    $versionInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($source)
    if ($versionInfo.ProductVersion -ne $version) { throw "Executable version does not match Cargo.toml: $($versionInfo.ProductVersion)" }
    $destination = Join-Path $repositoryPath 'dist'
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    $assetName = "CutPlanner-v$version.exe"
    $assetPath = Join-Path $destination $assetName
    Copy-Item -LiteralPath $source -Destination $assetPath -Force
    & python (Join-Path $PSScriptRoot 'verify_artifact.py') $assetPath --output (Join-Path $destination 'artifact-verification.json')
    if ($LASTEXITCODE -ne 0) { throw 'Executable resource/import checks failed.' }
    $digest = (Get-FileHash -LiteralPath $assetPath -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$assetPath.sha256", "$digest *$assetName`n", [Text.UTF8Encoding]::new($false))
    Copy-Item -LiteralPath (Join-Path $repositoryPath 'LICENSE') -Destination $destination -Force
    Copy-Item -LiteralPath (Join-Path $repositoryPath 'THIRD_PARTY_NOTICES.md') -Destination $destination -Force
    Write-Output "Prepared $assetPath and SHA-256. Nothing was executed or published."
} finally { Pop-Location }
