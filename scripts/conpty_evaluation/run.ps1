param(
    [Parameter(Mandatory = $true)][string]$EvaluationRoot,
    [ValidateSet('debug', 'release')][string]$Profile = 'debug',
    [ValidateSet('diagnostic', 'native-long', 'acceptance', 'hardening')][string]$Scenario = 'diagnostic',
    [string]$TargetDirectory,
    [switch]$BuildOnly
)

# Run only the explicitly prepared runtime, with child-local environment changes.
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$evaluation = [IO.Path]::GetFullPath($EvaluationRoot)
if ($evaluation -eq $repo -or $evaluation.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The evaluation directory must be outside the checkout.'
}
$config = Join-Path $evaluation 'evaluation.toml'
$runtime = Join-Path $evaluation 'runtime-x64/conpty.dll'
foreach ($path in @($config, $runtime, (Join-Path $evaluation 'runtime-x64/OpenConsole.exe'))) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw 'Run prepare.py first.' }
}
$run = Join-Path $evaluation ('run-{0}-{1}-{2}' -f (Get-Date -Format 'yyyyMMdd-HHmmss-fffffff'), $Profile, $Scenario)
New-Item -ItemType Directory -Path $run | Out-Null
$target = if ($TargetDirectory) { [IO.Path]::GetFullPath($TargetDirectory) } else { Join-Path $evaluation 'target' }
if ($target -eq $repo -or $target.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The target directory must be outside the checkout.'
}
$test = if ($Scenario -eq 'hardening') { 'hardening_gate' } else { 'tui_gate' }
$arguments = @('test', '--config', $config, '-p', 'relayterm-cli', '--test', $test)
if ($Profile -eq 'release') { $arguments += '--release' }
if ($Scenario -in @('diagnostic', 'native-long')) {
    $arguments += @('--features', 'relayterm-daemon/test-hooks')
}
if ($BuildOnly) {
    # A first build resolves the explicit path patch and updates Cargo.lock.
    $arguments += '--no-run'
    $deadline = 900
} else {
    $selector = switch ($Scenario) {
        'diagnostic' { 'sustained_diagnostic::compare_pipe_native_and_daemon' }
        'native-long' { 'sustained_diagnostic::compare_native_long' }
        'acceptance' { 'sustained_load::sustained_output_navigation_and_echo_meet_declared_contract' }
        'hardening' { 'tui::sustained_load::sustained_output_navigation_and_echo_meet_declared_contract' }
    }
    $arguments += @($selector, '--locked', '--', '--exact', '--nocapture', '--test-threads=1')
    if ($Scenario -in @('diagnostic', 'native-long')) { $arguments += '--ignored' }
    $deadline = if ($Scenario -in @('acceptance', 'hardening')) { 360 } else { 180 }
}
$info = [Diagnostics.ProcessStartInfo]::new('cargo')
$info.WorkingDirectory = $repo
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
foreach ($name in @('CI', 'RELAYTERM_TEST_RT', 'RELAYTERM_READER_METRICS', 'RELAYTERM_READER_DELAY_US')) {
    $info.Environment.Remove($name) | Out-Null
}
$info.Environment['CARGO_TARGET_DIR'] = $target
$info.Environment['RELAYTERM_EVALUATION_CONPTY'] = $runtime
if ($Scenario -in @('diagnostic', 'native-long')) { $info.Environment['RELAYTERM_READER_METRICS'] = $run }
foreach ($argument in $arguments) { $info.ArgumentList.Add($argument) }
$sources = foreach ($relative in @(
    'Cargo.lock',
    'crates/relayterm-cli/tests/tui_gate.rs',
    'crates/relayterm-cli/tests/support/sustained_load.rs',
    'crates/relayterm-cli/tests/support/sustained_diagnostic.rs',
    'crates/relayterm-daemon/src/supervisor.rs',
    'crates/relayterm-daemon/tests/pty_gate.rs',
    'crates/relayterm-terminal/src/lib.rs',
    'crates/relayterm-tui/src/lib.rs',
    'scripts/conpty_evaluation/run.ps1',
    'scripts/conpty_evaluation/runtime-adapter.patch'
)) {
    [ordered]@{ file = $relative; sha256 = (Get-FileHash -LiteralPath (Join-Path $repo $relative)).Hash.ToLowerInvariant() }
}
$sources | ConvertTo-Json | Set-Content (Join-Path $run 'sources-before.json')
$process = [Diagnostics.Process]::new()
$process.StartInfo = $info
$stdout = [IO.File]::Open((Join-Path $run 'stdout.log'), 'CreateNew', 'Write', 'Read')
$stderr = [IO.File]::Open((Join-Path $run 'stderr.log'), 'CreateNew', 'Write', 'Read')
$start = [DateTime]::UtcNow
$process.Start() | Out-Null
$outTask = $process.StandardOutput.BaseStream.CopyToAsync($stdout)
$errTask = $process.StandardError.BaseStream.CopyToAsync($stderr)
$timeout = -not $process.WaitForExit($deadline * 1000)
if ($timeout) {
    $process.Kill($true)
    if (-not $process.WaitForExit(5000)) { throw 'Owned process tree termination deadline.' }
}
$drained = $outTask.Wait(5000) -and $errTask.Wait(5000)
$stdout.Dispose()
$stderr.Dispose()
$executables = foreach ($file in @(Get-Item (Join-Path $target "$Profile/rt.exe") -ErrorAction SilentlyContinue) + @(Get-ChildItem (Join-Path $target "$Profile/deps/$test-*.exe") -ErrorAction SilentlyContinue)) {
    [ordered]@{ file = $file.Name; sha256 = (Get-FileHash -LiteralPath $file.FullName).Hash.ToLowerInvariant() }
}
$executables | ConvertTo-Json | Set-Content (Join-Path $run 'executables-after.json')
$result = [ordered]@{
    arguments = $arguments
    exit = $process.ExitCode
    timeout = $timeout
    output_drained = $drained
    elapsed_s = ([DateTime]::UtcNow - $start).TotalSeconds
    profile = $Profile
    scenario = $Scenario
    build_only = [bool]$BuildOnly
}
$result | ConvertTo-Json | Set-Content (Join-Path $run 'result.json')
Write-Output ($result | ConvertTo-Json -Compress)
if ($timeout -or -not $drained -or $process.ExitCode -ne 0) { exit 1 }
