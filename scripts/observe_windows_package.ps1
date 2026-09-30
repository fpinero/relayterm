param(
    [Parameter(Mandatory=$true)][string]$CandidateRoot,
    [Parameter(Mandatory=$true)][string]$ExpectedHash,
    [switch]$PrepareOnly
)
# Run only after the operator opens a normal real console. Never modify accounts.
$ErrorActionPreference='Stop'
function File-Hash([string]$Path) {
    $stream=[IO.File]::OpenRead($Path)
    $hash=[Security.Cryptography.SHA256]::Create()
    try { [BitConverter]::ToString($hash.ComputeHash($stream)).Replace('-','').ToLowerInvariant() }
    finally { $stream.Dispose(); $hash.Dispose() }
}
$binary=Join-Path $CandidateRoot 'rt.exe'
if ((File-Hash $binary) -cne $ExpectedHash.ToLowerInvariant()) { throw 'Candidate hash mismatch.' }
$identity=[Security.Principal.WindowsIdentity]::GetCurrent()
$principal=[Security.Principal.WindowsPrincipal]::new($identity)
$elevated=$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$groups=whoami.exe /groups /fo csv /nh
if ($LASTEXITCODE -ne 0) { throw 'Token group inspection failed.' }
$adminMember=(($groups | ConvertFrom-Csv -Header Name,Type,SID,Attributes).SID -contains 'S-1-5-32-544')
if ($elevated) { throw 'Use a non-elevated real console.' }
$standard=-not $adminMember
Write-Output ('M12 standard_account='+$standard.ToString().ToLowerInvariant())
Write-Output ('M12 candidate_sha256='+$ExpectedHash)
$scratch=Join-Path $env:TEMP ('rt-m12-observation-'+[Guid]::NewGuid().ToString('N'))
$project=Join-Path $scratch 'project'
$private=Join-Path $scratch 'private'
New-Item -ItemType Directory -Path $project | Out-Null
function Quote-Argument([string]$Argument) {
    if ($Argument.IndexOf([char]0) -ge 0) { throw 'NUL in command argument.' }
    '"'+[regex]::Replace([regex]::Replace($Argument, '(\\*)"', '$1$1\"'), '(\\+)$', '$1$1')+'"'
}
function Invoke-Admin([string[]]$Arguments) {
    # Read one complete JSON line and wait on the direct process, never pipe EOF.
    $start=[Diagnostics.ProcessStartInfo]::new()
    $start.FileName=$binary
    $values=@('--workspace',$project,'--home',$private,'--format','json','--timeout','5')+$Arguments
    $start.Arguments=(($values | ForEach-Object { Quote-Argument $_ }) -join ' ')
    $start.UseShellExecute=$false
    $start.CreateNoWindow=$true
    $start.RedirectStandardOutput=$true
    $start.RedirectStandardError=$true
    $start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
    $process=[Diagnostics.Process]::new()
    $process.StartInfo=$start
    try {
        if (-not $process.Start()) { throw 'Candidate launch failed.' }
        $line=$process.StandardOutput.ReadLineAsync()
        if (-not $line.Wait(15000) -or -not $process.WaitForExit(15000)) {
            if (-not $process.HasExited) { $process.Kill() }
            throw 'Direct candidate command exceeded its deadline.'
        }
        if ($process.ExitCode -ne 0) { throw ('Candidate command failed: '+$line.Result) }
        $value=$line.Result | ConvertFrom-Json
        if (-not $value.ok) { throw 'Candidate command rejected.' }
        return $value.result
    } finally { $process.Dispose() }
}
$null=Invoke-Admin -Arguments @('workspace','init','--name','M12 runtime observation')
$status=Invoke-Admin -Arguments @('workspace','status')
$definition=Join-Path $scratch 'definition.json'
$definitionValue=@{
    display_name='M12 runtime observation'
    command=(Join-Path $env:SystemRoot 'System32\cmd.exe')
    arguments=@('/D','/Q','/K','prompt M12$G')
    environment_allowlist=@()
    capabilities=@('interactive_terminal')
    enabled=$true
}
[IO.File]::WriteAllText($definition, ($definitionValue | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
$registered=Invoke-Admin -Arguments @('agent','register','--expected-revision',$status.revision,'--file',$definition)
$null=Invoke-Admin -Arguments @('session','create','--definition-id',$registered.entity_ids[0])
if ($PrepareOnly) {
    $null=Invoke-Admin -Arguments @('session','list')
    $null=Invoke-Admin -Arguments @('daemon','stop','--terminate-sessions')
    Write-Output 'M12 synthetic preparation passed; real-console observation pending.'
    return
}
Write-Output 'Press 3, then Enter. Type echo M12-RUNTIME-OK and press Enter.'
Write-Output 'Resize narrower and wider. Type exit and press Enter. Detach with Ctrl-], then Esc, then q.'
Write-Output 'Capture only the synthetic pane and the restored console area.'
& $binary --workspace $project --home $private
$tuiExit=$LASTEXITCODE
$null=Invoke-Admin -Arguments @('daemon','stop','--terminate-sessions')
@{ candidate_sha256=$ExpectedHash; standard_account=$standard; elevated=$elevated;
    tui_exit=$tuiExit; observation='Operator must report input, resize and restoration';
    private_state=$scratch } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $scratch 'operator-observation.json')
Write-Output ('M12 tui_exit='+$tuiExit)
Write-Output 'Private state retained. Report the observations and cropped screenshots.'
