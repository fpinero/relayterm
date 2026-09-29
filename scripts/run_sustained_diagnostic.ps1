param(
    [Parameter(Mandatory = $true)][string]$ArtifactRoot,
    [ValidateSet('debug', 'release')][string]$Profile = 'debug',
    [ValidateSet('baseline', 'native-writer', 'read-batching', 'larger-writes', 'daemon-batching', 'native-long')]
    [string]$Mode = 'baseline'
)

# Keep diagnostic builds, raw output and identifiers outside the public checkout.
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$artifactPath = [IO.Path]::GetFullPath($ArtifactRoot)
if ($artifactPath.TrimEnd('\', '/') -eq $repo.TrimEnd('\', '/') -or
    $artifactPath.StartsWith($repo.TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Choose an artifact directory outside the checkout.'
}
$runName = '{0}-{1}-{2}' -f (Get-Date -Format 'yyyyMMdd-HHmmss-fffffff'), $Profile, $Mode
$runPath = Join-Path $artifactPath $runName
New-Item -ItemType Directory -Path $runPath -ErrorAction Stop | Out-Null
$cache = Join-Path $artifactPath 'target'

function Invoke-BoundedCargo([string]$Name, [string[]]$Arguments, [int]$DeadlineSeconds) {
    $info = [Diagnostics.ProcessStartInfo]::new('cargo')
    $info.WorkingDirectory = $repo
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.Environment.Remove('CI') | Out-Null
    $info.Environment.Remove('RELAYTERM_TEST_RT') | Out-Null
    $info.Environment['CARGO_TARGET_DIR'] = $cache
    $info.Environment['RELAYTERM_READER_METRICS'] = $runPath
    $info.Environment['RELAYTERM_READER_DELAY_US'] = if ($Mode -eq 'daemon-batching') { '1000' } else { '0' }
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    $stdout = [IO.File]::Open((Join-Path $runPath "$Name.stdout.log"), 'CreateNew', 'Write', 'Read')
    $stderr = [IO.File]::Open((Join-Path $runPath "$Name.stderr.log"), 'CreateNew', 'Write', 'Read')
    $started = [DateTime]::UtcNow
    $process.Start() | Out-Null
    $stdoutTask = $process.StandardOutput.BaseStream.CopyToAsync($stdout)
    $stderrTask = $process.StandardError.BaseStream.CopyToAsync($stderr)
    $timeout = -not $process.WaitForExit($DeadlineSeconds * 1000)
    if ($timeout) {
        $process.Kill($true)
        if (-not $process.WaitForExit(5000)) { throw 'Owned process tree did not exit after termination.' }
    }
    $drained = $stdoutTask.Wait(5000) -and $stderrTask.Wait(5000)
    $stdout.Dispose()
    $stderr.Dispose()
    $result = [ordered]@{
        command = 'cargo ' + ($Arguments -join ' ')
        exit = $process.ExitCode
        timeout = $timeout
        output_drained = $drained
        elapsed_s = ([DateTime]::UtcNow - $started).TotalSeconds
        profile = $Profile
        mode = $Mode
        CI_present = $false
        product_override_present = $false
    }
    $result | ConvertTo-Json | Set-Content (Join-Path $runPath "$Name.result.json")
    Write-Host ($result | ConvertTo-Json -Compress)
    return ($result.exit -eq 0 -and -not $timeout -and $drained)
}

$arguments = @('test', '-p', 'relayterm-cli', '--test', 'tui_gate', '--features', 'relayterm-daemon/test-hooks', '--locked')
if ($Profile -eq 'release') { $arguments += '--release' }
if (-not (Invoke-BoundedCargo 'build' ($arguments + '--no-run') 900)) { exit 1 }

$sources = foreach ($relative in @(
    'crates/relayterm-cli/tests/tui_gate.rs',
    'crates/relayterm-cli/tests/support/sustained_load.rs',
    'crates/relayterm-cli/tests/support/sustained_diagnostic.rs',
    'crates/relayterm-daemon/src/supervisor.rs',
    'scripts/run_sustained_diagnostic.ps1'
)) {
    [ordered]@{ file = $relative; sha256 = (Get-FileHash -LiteralPath (Join-Path $repo $relative)).Hash.ToLowerInvariant() }
}
$sources | ConvertTo-Json | Set-Content (Join-Path $runPath 'sources.json')
$executables = foreach ($file in @(Get-Item (Join-Path $cache "$Profile/rt.exe")) + @(Get-ChildItem (Join-Path $cache "$Profile/deps/tui_gate-*.exe"))) {
    [ordered]@{ file = $file.Name; sha256 = (Get-FileHash -LiteralPath $file.FullName).Hash.ToLowerInvariant() }
}
$executables | ConvertTo-Json | Set-Content (Join-Path $runPath 'executables.json')
$selector = switch ($Mode) {
    'native-writer' { 'compare_native_writer' }
    'read-batching' { 'compare_read_batching' }
    'larger-writes' { 'compare_larger_writes' }
    'native-long' { 'compare_native_long' }
    default { 'compare_pipe_native_and_daemon' }
}
if (-not (Invoke-BoundedCargo 'diagnostic' ($arguments + @("sustained_diagnostic::$selector", '--', '--exact', '--ignored', '--nocapture', '--test-threads=1')) 180)) {
    exit 1
}
if ($Mode -notin @('read-batching', 'native-long')) {
    $readerReports = @(Get-ChildItem -LiteralPath $runPath -Filter 'reader-*.json')
    if ($readerReports.Count -ne 1) { throw 'Expected exactly one daemon reader report.' }
    $reader = Get-Content -LiteralPath $readerReports[0].FullName -Raw | ConvertFrom-Json
    if ($reader.bytes -le 0 -or $reader.reads -le 0 -or $null -eq $reader.process_us) {
        throw 'Daemon reader measurements are incomplete.'
    }
}
Write-Host 'Diagnostic completed. Exit zero records completion, not sustained acceptance.'
