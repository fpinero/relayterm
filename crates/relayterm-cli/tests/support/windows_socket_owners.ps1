param(
    [Parameter(Mandatory = $true)]
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$owners = @(
    Get-NetTCPConnection | ForEach-Object { $_.OwningProcess.ToString() }
    Get-NetUDPEndpoint | ForEach-Object { $_.OwningProcess.ToString() }
) | Sort-Object -Unique

[System.IO.File]::WriteAllLines($OutputPath, [string[]]@($owners))
