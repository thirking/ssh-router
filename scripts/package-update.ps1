param(
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$Installer,
    [Parameter(Mandatory = $true)][string]$DownloadUrl,
    [string]$Notes = "",
    [string]$OutputDirectory = "publish",
    [string]$Packager = "target/release/ssh-router-update-packager.exe"
)

$ErrorActionPreference = "Stop"
if (-not $env:SSH_ROUTER_SIGNING_PRIVATE_KEY) {
    throw "SSH_ROUTER_SIGNING_PRIVATE_KEY is required"
}

$output = New-Item -ItemType Directory -Force -Path $OutputDirectory
$signature = Join-Path $output.FullName ((Split-Path $Installer -Leaf) + ".sig")
$manifest = Join-Path $output.FullName "latest.json"
$publishedAt = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")

if (-not (Test-Path $Packager)) { throw "Update packager not found: $Packager" }
& $Packager `
    --installer $Installer `
    --signature $signature `
    --manifest $manifest `
    --version $Version `
    --url $DownloadUrl `
    --notes $Notes `
    --published-at $publishedAt
if ($LASTEXITCODE -ne 0) {
    throw "Failed to sign update package"
}

Write-Host "Created $signature"
Write-Host "Created $manifest"
