param(
    [Parameter(Mandatory = $true)][string]$ReleaseRoot,
    [Parameter(Mandatory = $true)][string]$Destination
)

$ErrorActionPreference = "Stop"
$source = Join-Path $ReleaseRoot "rt.exe"
$target = Join-Path $Destination "rt.exe"
if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
    throw "Installation refused: the source rt.exe is not a regular file."
}
$releaseItem = Get-Item -LiteralPath $ReleaseRoot
$sourceItem = Get-Item -LiteralPath $source
if (($releaseItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
    ($sourceItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Installation refused: the release root and executable must not be links."
}
$required = @("LICENSE", "THIRD_PARTY_NOTICES.txt", "THIRD_PARTY_LICENSES.txt", "INSTALL.md", "RECOVERY.md", "manifest.json", "install_release.sh", "install_release.ps1")
foreach ($name in $required) {
    $itemPath = Join-Path $ReleaseRoot $name
    if (-not (Test-Path -LiteralPath $itemPath -PathType Leaf)) {
        throw "Installation refused: the extracted release inventory is incomplete."
    }
    if (((Get-Item -LiteralPath $itemPath).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Installation refused: release inventory entries must not be links."
    }
}
if (Test-Path -LiteralPath $target) {
    throw "Installation refused: the destination rt.exe already exists."
}
if (-not (Test-Path -LiteralPath $Destination -PathType Container)) {
    throw "Installation refused: the destination directory must already exist."
}
if (((Get-Item -LiteralPath $Destination).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Installation refused: the destination directory must not be a link."
}
$temporary = Join-Path $Destination (".rt-install-" + [Guid]::NewGuid().ToString("N") + ".exe")
try {
    Copy-Item -LiteralPath $source -Destination $temporary -ErrorAction Stop
    Move-Item -LiteralPath $temporary -Destination $target -ErrorAction Stop
} finally {
    if (Test-Path -LiteralPath $temporary) {
        Remove-Item -LiteralPath $temporary -Force
    }
}
Write-Output ("installed: " + $target)
