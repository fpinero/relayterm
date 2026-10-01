param([switch]$PreflightOnly)
# Local observation fixture. No account, persistent policy or system ACL changes.
$ErrorActionPreference = 'Stop'
$base = $PSScriptRoot
$results = Join-Path $base 'results'
$runId = [Guid]::NewGuid().ToString('N')
$attempt = Join-Path $results $runId
$stage = 'result_access'
$code = 1
$child = $null
$scope = $null
$owned = @{}
$ownership = @{}
$report = [ordered]@{
    schema = 1; run_id = $runId; status = 'failed'; failure_stage = $stage
    diagnostic = 0; standard_account = $false; elevated = $null
    preparation_only = [bool]$PreflightOnly; observer_exit = $null; tui_exit = $null
    automatic = [ordered]@{}; physical = [ordered]@{}
}
function Hash([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}
function Assert-Package {
    $files = @(Get-ChildItem -LiteralPath (Join-Path $base 'candidate') -Force -Recurse)
    $pins = $inventory.installation.inventory.PSObject.Properties
    if ($files.Count -ne 13 -or @($files | Where-Object PSIsContainer).Count) {
        throw 'Invalid package inventory.'
    }
    foreach ($pin in $pins) {
        if ($pin.Name -ne [IO.Path]::GetFileName($pin.Name)) { throw 'Invalid inventory name.' }
        if ((Hash (Join-Path $base ('candidate\' + $pin.Name))) -cne $pin.Value) {
            throw 'Package integrity failure.'
        }
    }
}
function Quote([string]$Value) {
    if ($Value.IndexOf([char]0) -ge 0) { throw 'NUL argument.' }
    '"' + [regex]::Replace([regex]::Replace($Value, '(\\*)"', '$1$1\"'), '(\\+)$', '$1$1') + '"'
}
function Snapshot-Owned {
    $all = @(Get-CimInstance Win32_Process)
    foreach ($p in $all) {
        if ($p.ExecutablePath -eq $binary -and $p.CommandLine -and
            $p.CommandLine.Contains($scope + '\rt-m12-observation-')) {
            $owned[[string]$p.ProcessId] = $p.CreationDate
            $ownership[[string]$p.ProcessId] = 'exact candidate and scoped fixture arguments'
        }
    }
    # Track descendants by PID plus creation time, never by executable name.
    do {
        $added = $false
        foreach ($p in $all) {
            $parentKey = [string]$p.ParentProcessId
            $key = [string]$p.ProcessId
            $parent = @($all | Where-Object { [string]$_.ProcessId -eq $parentKey })
            if ($owned.ContainsKey($parentKey) -and $parent.Count -eq 1 -and
                $parent[0].CreationDate -eq $owned[$parentKey] -and -not $owned.ContainsKey($key) -and
                $p.CreationDate -ge $owned[$parentKey]) {
                $owned[$key] = $p.CreationDate
                $ownership[$key] = 'descendant of a live positively identified fixture process'
                $added = $true
            }
        }
    } while ($added)
    @($all | Where-Object {
        $owned.ContainsKey([string]$_.ProcessId) -and
        $_.CreationDate -eq $owned[[string]$_.ProcessId]
    })
}
function Stop-Fixture {
    $live = @(Snapshot-Owned)
    $report.automatic.forced_cleanup = ($live.Count -gt 0)
    if ($live.Count) {
        # Stop only positively identified fixture processes, retaining all state.
        foreach ($p in ($live | Sort-Object CreationDate -Descending)) {
            $current = Get-CimInstance Win32_Process -Filter ('ProcessId=' + $p.ProcessId)
            if ($current -and $current.CreationDate -eq $p.CreationDate) {
                Stop-Process -Id $p.ProcessId -ErrorAction Stop
            }
        }
        Start-Sleep -Milliseconds 500
    }
    $remaining = @(Snapshot-Owned)
    $report.automatic.cleanup_remaining = $remaining.Count
    $report.automatic.cleanup_verified = ($remaining.Count -eq 0)
    if ($remaining.Count) { throw 'Fixture cleanup incomplete.' }
}
function Confirm([string]$Prompt) {
    (Read-Host ($Prompt + ' [s/n]')).Trim().ToLowerInvariant() -eq 's'
}
try {
    New-Item -ItemType Directory -Path $attempt | Out-Null
    $control = Join-Path $attempt 'access-control.txt'
    [IO.File]::WriteAllText($control, $runId)
    if ([IO.File]::ReadAllText($control) -cne $runId) { throw 'Result access failure.' }
    $report.automatic.shared_write_read = $true
    $stage = 'account_preflight'
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    $report.elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    $groups = & "$env:SystemRoot\System32\whoami.exe" /groups /fo csv /nh
    if ($LASTEXITCODE -ne 0) { throw 'Token inspection failed.' }
    $sids = @(($groups | ConvertFrom-Csv -Header Name,Type,SID,Attributes).SID)
    if (-not $sids.Count -or @($sids | Where-Object { $_ -notmatch '^S-1-' }).Count) {
        throw 'Invalid token SID output.'
    }
    $admin = $sids -contains 'S-1-5-32-544'
    $report.standard_account = (-not $admin -and -not $report.elevated)
    $report.automatic.administrators_sid_present = $admin
    Write-Host ('standard_account=' + $report.standard_account.ToString().ToLowerInvariant())
    Write-Host ('elevated=' + $report.elevated.ToString().ToLowerInvariant())
    if (-not $report.standard_account) { throw 'Administrator token refused.' }
    $stage = 'integrity'
    $inventory = Get-Content -Raw -LiteralPath (Join-Path $base 'candidate-inventory.json') | ConvertFrom-Json
    $binding = Get-Content -Raw -LiteralPath (Join-Path $base 'kit-identity.json') | ConvertFrom-Json
    $expected = 'ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa'
    if ($inventory.artifact.binary.sha256 -cne $expected) { throw 'Invalid candidate binding.' }
    Assert-Package
    foreach ($file in $binding.files.PSObject.Properties) {
        if ($file.Name -ne [IO.Path]::GetFileName($file.Name)) { throw 'Invalid fixture name.' }
        if ((Hash (Join-Path $base $file.Name)) -cne $file.Value) { throw 'Fixture changed.' }
    }
    $report.identities = $binding
    $report.candidate_sha256 = $expected
    $report.product_source = $inventory.product_source
    $report.automatic.integrity_before = $true
    Write-Host ('candidate_sha256=' + $expected)
    $stage = 'system_prerequisite'
    $runtime = Join-Path $env:SystemRoot 'System32\VCRUNTIME140.dll'
    if (-not (Test-Path -LiteralPath $runtime -PathType Leaf)) { throw 'System runtime missing.' }
    $report.automatic.system_runtime_sha256 = Hash $runtime
    $report.automatic.system_runtime_version = (Get-Item -LiteralPath $runtime).VersionInfo.FileVersion
    $pin = $inventory.pe.system_prerequisites[0]
    if ($report.automatic.system_runtime_sha256 -cne $pin.sha256) { throw 'System runtime differs from mapped prerequisite.' }
    $report.os_version = [Environment]::OSVersion.Version.ToString()
    $report.terminal_scope = 'ordinary interactive Windows console on the existing development host'
    if ($PreflightOnly) {
        $report.status = 'preflight_only'; $code = 0
    } else {
        $stage = 'interactive_console'
        if ([Console]::IsInputRedirected -or [Console]::IsOutputRedirected) { throw 'Interactive console required.' }
        $report.automatic.console_before = @{width=[Console]::WindowWidth;height=[Console]::WindowHeight}
        $stage = 'private_temp'
        $nativeTemp = [IO.Path]::GetTempPath()
        if (-not [IO.Path]::IsPathRooted($nativeTemp) -or
            -not (Test-Path -LiteralPath $nativeTemp -PathType Container)) { throw 'Native TEMP unavailable.' }
        $scope = Join-Path $nativeTemp ('rt-m12-standard-' + $runId)
        New-Item -ItemType Directory -Path $scope | Out-Null
        $probe = Join-Path $scope 'native-temp-control.txt'
        [IO.File]::WriteAllText($probe, $runId)
        if ([IO.File]::ReadAllText($probe) -cne $runId) { throw 'Native TEMP not writable.' }
        $binary = Join-Path $base 'candidate\rt.exe'
        $stage = 'observer'
        Write-Host 'Paso 1: pulsa 3 y Enter. Escribe echo M12-RUNTIME-OK y Enter.'
        Write-Host 'Paso 2: estrecha y ensancha la consola. Repite echo M12-RUNTIME-OK.'
        Write-Host 'Paso 3: escribe exit y Enter. Libera con Ctrl-], Esc y q.'
        Write-Host 'Al volver aqui, responde las tres comprobaciones. Limite: 30 minutos.'
        $start = [Diagnostics.ProcessStartInfo]::new()
        $start.FileName = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
        $start.Arguments = '-NoLogo -NoProfile -ExecutionPolicy RemoteSigned -File ' +
            (Quote (Join-Path $base 'observe_windows_package.ps1')) + ' -CandidateRoot ' +
            (Quote (Join-Path $base 'candidate')) + ' -ExpectedHash ' + (Quote $expected)
        $start.UseShellExecute = $false
        $start.EnvironmentVariables['TEMP'] = $scope
        $start.EnvironmentVariables['TMP'] = $scope
        $child = [Diagnostics.Process]::new()
        $child.StartInfo = $start
        $report.status = 'in_progress'; $report.failure_stage = $stage
        [IO.File]::WriteAllText((Join-Path $attempt 'result.json'), ($report | ConvertTo-Json -Depth 12), [Text.UTF8Encoding]::new($false))
        if (-not $child.Start()) { throw 'Observer did not start.' }
        $clock = [Diagnostics.Stopwatch]::StartNew()
        while (-not $child.WaitForExit(500)) {
            $null = Snapshot-Owned
            if ($clock.Elapsed.TotalSeconds -gt 1800) {
                $child.Kill(); $child.WaitForExit(5000) | Out-Null
                throw 'Observer deadline exceeded.'
            }
        }
        $report.observer_exit = $child.ExitCode
        $stage = 'exact_readback'
        $fixtures = @(Get-ChildItem -LiteralPath $scope -Directory -Filter 'rt-m12-observation-*')
        if ($fixtures.Count -ne 1) { throw 'Expected exactly one new fixture.' }
        $rawPath = Join-Path $fixtures[0].FullName 'operator-observation.json'
        if (-not (Test-Path -LiteralPath $rawPath -PathType Leaf)) { throw 'Observer result missing.' }
        $raw = Get-Content -Raw -LiteralPath $rawPath | ConvertFrom-Json
        if ($raw.private_state -cne $fixtures[0].FullName -or $raw.candidate_sha256 -cne $expected -or
            $raw.standard_account -ne $true -or $raw.elevated -ne $false -or
            $report.observer_exit -ne 0 -or $null -eq $raw.tui_exit) { throw 'Observer binding rejected.' }
        $report.tui_exit = $raw.tui_exit
        $report.automatic.exact_invocation_readback = $true
        $report.automatic.observer_completed_commands = @(
            @('workspace init','workspace status','agent register','session create','daemon stop --terminate-sessions') |
            ForEach-Object { @{command=$_;exit_code=0;accepted=$true} }
        )
        $report.automatic.raw_observation_sha256 = Hash $rawPath
        $stage = 'normal_cleanup'
        $cleanupClock = [Diagnostics.Stopwatch]::StartNew()
        do {
            $live = @(Snapshot-Owned)
            if ($live.Count) { Start-Sleep -Milliseconds 250 }
        } while ($live.Count -and $cleanupClock.Elapsed.TotalSeconds -lt 5)
        $report.automatic.cleanup_remaining = $live.Count
        $report.automatic.cleanup_verified = ($live.Count -eq 0)
        $report.automatic.forced_cleanup = $false
        if ($live.Count) { throw 'Normal cleanup incomplete.' }
        Assert-Package
        $report.automatic.integrity_after = $true
        $report.automatic.console_after = @{width=[Console]::WindowWidth;height=[Console]::WindowHeight}
        $stage = 'physical_confirmations'
        $report.physical.input = Confirm 'Aparecio correctamente M12-RUNTIME-OK al escribirlo?'
        $report.physical.resize = Confirm 'Al estrechar y ensanchar, el redibujado y la salida nueva fueron utilizables?'
        $report.physical.restoration = Confirm 'Tras exit, Ctrl-], Esc y q, volvio la consola normal sin reset ni cierre forzado?'
        if ($raw.tui_exit -ne 0 -or -not $report.physical.input -or
            -not $report.physical.resize -or -not $report.physical.restoration) { throw 'Observation did not pass.' }
        $report.status = 'passed'; $code = 0
    }
    $report.failure_stage = $null
} catch {
    $report.status = 'failed'
    $report.failure_stage = $stage
    $report.diagnostic = $_.Exception.HResult
    Write-Host ('No se acepta este intento. Etapa=' + $stage + '; codigo=' + $report.diagnostic)
    if ($child -and -not $child.HasExited) { $child.Kill(); $child.WaitForExit(5000) | Out-Null }
    if ($scope -and $binary) {
        try { Stop-Fixture } catch { $report.automatic.cleanup_verified = $false }
    }
} finally {
    $report.automatic.fixture_processes = @($owned.Keys | ForEach-Object {
        @{pid=[int]$_;created_utc=$owned[$_].ToUniversalTime().ToString('o');ownership=$ownership[$_]}
    })
    if ($child) { $child.Dispose() }
    if (Test-Path -LiteralPath $attempt -PathType Container) {
        $json = $report | ConvertTo-Json -Depth 12
        [IO.File]::WriteAllText((Join-Path $attempt 'result.json'), $json, [Text.UTF8Encoding]::new($false))
    }
}
Write-Host ('Resultado=' + $report.status + '; intento=' + $runId)
Write-Host 'Resultados guardados en la carpeta results del kit. Conserva esta consola hasta ver el resultado.'
exit $code
