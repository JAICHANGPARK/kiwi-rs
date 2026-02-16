param(
  [string]$Version = "latest",
  [string]$Prefix = "",
  [ValidateSet("x64", "Win32")]
  [string]$Arch = "x64",
  [string]$ModelVariant = "base"
)

$ErrorActionPreference = "Stop"

function Resolve-Tag {
  param([string]$RequestedVersion)

  if ($RequestedVersion -ne "latest") {
    if ($RequestedVersion.StartsWith("v")) {
      return $RequestedVersion
    }
    return "v$RequestedVersion"
  }

  $apiUrl = "https://api.github.com/repos/bab2min/Kiwi/releases/latest"
  $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "kiwi-rs-installer" }
  if (-not $release.tag_name) {
    throw "Could not resolve latest Kiwi release tag from GitHub API."
  }
  return $release.tag_name
}

if (-not $Prefix) {
  if ($env:KIWI_PREFIX) {
    $Prefix = $env:KIWI_PREFIX
  } elseif ($env:LOCALAPPDATA) {
    $Prefix = Join-Path $env:LOCALAPPDATA "kiwi"
  } else {
    throw "LOCALAPPDATA is not set. Pass -Prefix explicitly."
  }
}

$tag = Resolve-Tag $Version
$versionNoV = $tag.TrimStart("v")

$libAsset = "kiwi_win_${Arch}_v${versionNoV}.zip"
$modelAsset = "kiwi_model_v${versionNoV}_${ModelVariant}.tgz"
$baseUrl = "https://github.com/bab2min/Kiwi/releases/download/$tag"

$tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ("kiwi-rs-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
  $libArchive = Join-Path $tmpDir $libAsset
  $modelArchive = Join-Path $tmpDir $modelAsset

  Write-Host "[kiwi-rs] Downloading $libAsset"
  Invoke-WebRequest -Uri "$baseUrl/$libAsset" -OutFile $libArchive

  Write-Host "[kiwi-rs] Downloading $modelAsset"
  Invoke-WebRequest -Uri "$baseUrl/$modelAsset" -OutFile $modelArchive

  New-Item -ItemType Directory -Path $Prefix -Force | Out-Null

  Write-Host "[kiwi-rs] Extracting library archive"
  Expand-Archive -Path $libArchive -DestinationPath $Prefix -Force

  $tarCmd = Get-Command tar -ErrorAction SilentlyContinue
  if (-not $tarCmd) {
    throw "tar command is required to extract model archive. Install bsdtar or Git for Windows."
  }

  Write-Host "[kiwi-rs] Extracting model archive"
  & $tarCmd.Source -xzf $modelArchive -C $Prefix
  if ($LASTEXITCODE -ne 0) {
    throw "tar extraction failed for model archive."
  }
}
finally {
  Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
}

$libPath = Join-Path $Prefix "lib\\kiwi.dll"
$modelPath = Join-Path $Prefix "models\\cong\\$ModelVariant"

Write-Host ""
Write-Host "[kiwi-rs] Install done."
Write-Host "[kiwi-rs] Library path: $libPath"
Write-Host "[kiwi-rs] Model path:   $modelPath"
Write-Host ""
Write-Host "[kiwi-rs] Optional env setup:"
Write-Host "  `$env:KIWI_LIBRARY_PATH = `"$libPath`""
Write-Host "  `$env:KIWI_MODEL_PATH = `"$modelPath`""

