# qatest Windows installer. Verifies SHA-256 from the same latest.json as `qatest update`.
$ErrorActionPreference = "Stop"
$Bin = "qatest.exe"
$InstallDir = if ($env:QATEST_INSTALL_DIR) { $env:QATEST_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "qatest\bin" }
$Target = "windows-x86_64"

Write-Host "qatest installer  github.com/Simonethg/qatest"

$urls = @(
  "https://qatest.sh/latest.json",
  "https://github.com/Simonethg/qatest/releases/latest/download/latest.json",
  "https://raw.githubusercontent.com/Simonethg/qatest/main/latest.json"
)
if ($env:QATEST_MANIFEST_URL) { $urls = @($env:QATEST_MANIFEST_URL) }

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

$tmp = Join-Path $env:TEMP "qatest-install.zip"
Invoke-WebRequest -Uri $url -OutFile $tmp -TimeoutSec 120
$hash = (Get-FileHash -Path $tmp -Algorithm SHA256).Hash.ToLower()
if ($hash -ne $sha) { throw "checksum mismatch" }

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$destZip = Join-Path $env:TEMP "qatest-unpack"
if (Test-Path $destZip) { Remove-Item -Recurse -Force $destZip }
Expand-Archive -Path $tmp -DestinationPath $destZip -Force
Copy-Item -Force (Join-Path $destZip $Bin) (Join-Path $InstallDir $Bin)
Remove-Item -Force $tmp

Write-Host "> installed $(Join-Path $InstallDir $Bin)"
Write-Host "> ready. run 'qatest' to get started."
