param([string]$ReleaseRoot, [Parameter(Mandatory=$true)][string]$Destination, [switch]$Remove)
# Fixed owned inventory; never edit PATH, host policy or private workspace state.
$ErrorActionPreference='Stop'
$owned=@('rt.exe','conpty.dll','OpenConsole.exe','CONPTY_LICENSE.txt','CONPTY_PROVENANCE.json','LICENSE','THIRD_PARTY_NOTICES.txt','THIRD_PARTY_LICENSES.txt','INSTALL.md','RECOVERY.md','manifest.json','install_release.sh','install_release.ps1')
function File-Hash([string]$Path) {
    $stream=[IO.File]::OpenRead($Path)
    $hash=[Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($hash.ComputeHash($stream)).Replace('-','').ToLowerInvariant() }
    finally { $stream.Dispose(); $hash.Dispose() }
}
function Assert-Directory([string]$Path) {
    $item=Get-Item -LiteralPath $Path
    if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Directory must be regular.' }
}
function Assert-Inventory([string]$Root) {
    Assert-Directory $Root
    foreach ($name in $owned) {
        $item=Get-Item -LiteralPath (Join-Path $Root $name)
        if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Inventory must contain regular files.' }
    }
    $manifest=Get-Content -Raw -LiteralPath (Join-Path $Root 'manifest.json') | ConvertFrom-Json
    if ($manifest.product -ne 'relayterm' -or $manifest.target -ne 'x86_64-pc-windows-msvc' -or $manifest.format_version -ne 1 -or $manifest.contents.Count -ne ($owned.Count-1)) { throw 'Unexpected manifest.' }
    $seen=@{}
    foreach ($entry in $manifest.contents) {
        if ($entry.path -eq 'manifest.json' -or $owned -cnotcontains $entry.path -or $seen.ContainsKey($entry.path)) { throw 'Unexpected owned path.' }
        $seen[$entry.path]=$true
        $path=Join-Path $Root $entry.path
        if ((Get-Item -LiteralPath $path).Length -ne $entry.bytes -or (File-Hash $path) -cne $entry.sha256) { throw 'Inventory hash mismatch.' }
    }
    foreach ($pair in @(@('conpty.dll','39fba2713e2495117b1591ae8c32a3b904bea7aa66069cf7815e2844c76d75d8'),@('OpenConsole.exe','b7fd936c2668b87b9ecf7b3366dc6568afc1c6f981874cba3e955a1c35cf8160'))) {
        if ((File-Hash (Join-Path $Root $pair[0])) -cne $pair[1]) { throw 'Pinned runtime hash mismatch.' }
    }
}
Assert-Directory $Destination
if ($Remove) {
    Assert-Inventory $Destination
    foreach ($name in $owned) { Remove-Item -LiteralPath (Join-Path $Destination $name) }
    Write-Output 'Removed verified owned files; unrelated files and state preserved.'
    return
}
Assert-Inventory $ReleaseRoot
foreach ($name in $owned) { if (Test-Path -LiteralPath (Join-Path $Destination $name)) { throw ('Destination collision: '+$name) } }
$staging=Join-Path $Destination ('.rt-install-'+[Guid]::NewGuid().ToString('N'))
$installed=[Collections.Generic.List[string]]::new()
New-Item -ItemType Directory -Path $staging | Out-Null
try {
    foreach ($name in $owned) { Copy-Item -LiteralPath (Join-Path $ReleaseRoot $name) -Destination (Join-Path $staging $name) }
    Assert-Inventory $staging
    # Install rt.exe last, and refuse races instead of replacing a collision.
    foreach ($name in @($owned | Where-Object { $_ -ne 'rt.exe' })+@('rt.exe')) {
        $target=Join-Path $Destination $name
        [IO.File]::Move((Join-Path $staging $name),$target)
        $installed.Add($target)
    }
} catch {
    foreach ($path in $installed) { Remove-Item -LiteralPath $path }
    throw
} finally {
    foreach ($name in $owned) { $path=Join-Path $staging $name; if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path } }
    Remove-Item -LiteralPath $staging
}
Write-Output ('Installed verified Windows package: '+(Join-Path $Destination 'rt.exe'))
