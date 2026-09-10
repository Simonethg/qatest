# qat Windows installer. Verifies SHA-256 from the same latest.json as `qat update`.
$ErrorActionPreference = "Stop"
$Bin = "qat.exe"
$InstallDir = if ($env:QAT_INSTALL_DIR) { $env:QAT_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "qat\bin" }
$Target = "windows-x86_64"

Write-Host "qat installer  github.com/Simonethg/qat"

$urls = @(
  "https://qat.sh/latest.json",
  "https://getqat.dev/latest.json",
  "https://github.com/Simonethg/qat/releases/latest/download/latest.json",
  "https://raw.githubusercontent.com/Simonethg/qat/main/latest.json"
)
if ($env:QAT_MANIFEST_URL) { $urls = @($env:QAT_MANIFEST_URL) }

$manifest = $null
foreach ($u in $urls) {
  try {
    $manifest = Invoke-RestMethod -Uri $u -TimeoutSec 20
    Write-Host "> manifest $u"
    break
  } catch { }
}
if (-not $manifest) { throw "can't reach latest.json" }

$url = $manifest.assets.$Target
$sha = ([string]$manifest.sha256.$Target).ToLower()
if (-not $url) { throw "no asset for $Target" }
if ($sha.Length -ne 64) { throw "invalid sha256" }

$tmp = Join-Path $env:TEMP "qat-install.zip"
Invoke-WebRequest -Uri $url -OutFile $tmp -TimeoutSec 120
$hash = (Get-FileHash -Path $tmp -Algorithm SHA256).Hash.ToLower()
if ($hash -ne $sha) { throw "checksum mismatch" }

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$destZip = Join-Path $env:TEMP "qat-unpack"
if (Test-Path $destZip) { Remove-Item -Recurse -Force $destZip }
Expand-Archive -Path $tmp -DestinationPath $destZip -Force
Copy-Item -Force (Join-Path $destZip $Bin) (Join-Path $InstallDir $Bin)
Remove-Item -Force $tmp

Write-Host "> installed $(Join-Path $InstallDir $Bin)"
Write-Host "> ready. run 'qat' to get started."
