param(
    [Parameter(Mandatory = $true)][int]$MotorPid,
    [Parameter(Mandatory = $true)][string]$ResultPath,
    [Parameter(Mandatory = $true)][string]$Prefix
)

$ErrorActionPreference = 'Stop'
$process = Get-Process -Id $MotorPid -ErrorAction Stop
if ($process.ProcessName -ne 'MotHor') { throw "PID $MotorPid no es MotHor" }
$source = (Resolve-Path -LiteralPath $ResultPath).Path
$expectedMotor = Join-Path (Split-Path -Parent $source) 'MotHor.exe'
if ($process.Path -ine $expectedMotor) {
    throw "El PID $MotorPid no corresponde al motor del resultado $source"
}
$destination = [System.IO.Path]::GetFullPath("$Prefix.xrho")
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path.TrimEnd('\') + '\'
if (-not $destination.StartsWith($workspace, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "La captura debe quedar dentro de $workspace"
}
if (Test-Path -LiteralPath $destination) { throw "La captura ya existe: $destination" }

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class GhcSnapshotThreads {
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern IntPtr OpenThread(uint access, bool inherit, uint id);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern uint SuspendThread(IntPtr thread);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern uint ResumeThread(IntPtr thread);
    [DllImport("kernel32.dll")]
    public static extern bool CloseHandle(IntPtr handle);
}
'@

$handles = [System.Collections.Generic.List[System.IntPtr]]::new()
try {
    foreach ($thread in $process.Threads) {
        $handle = [GhcSnapshotThreads]::OpenThread(0x0002, $false, [uint32]$thread.Id)
        if ($handle -eq [IntPtr]::Zero) { throw "OpenThread falló para $($thread.Id)" }
        $previous = [GhcSnapshotThreads]::SuspendThread($handle)
        if ($previous -eq [uint32]::MaxValue) {
            [void][GhcSnapshotThreads]::CloseHandle($handle)
            throw "SuspendThread falló para $($thread.Id)"
        }
        $handles.Add($handle)
    }
    Copy-Item -LiteralPath $source -Destination $destination -Force
    & python (Join-Path $PSScriptRoot 'inspect_native_scoring_records.py') $MotorPid --dump-prefix $Prefix |
        Out-File -LiteralPath "$Prefix-records.json" -Encoding utf8
    if ($LASTEXITCODE -ne 0) { throw 'No se pudieron leer los registros' }
    & python (Join-Path $PSScriptRoot 'scan_scoring_object.py') $MotorPid --dump-prefix $Prefix |
        Out-File -LiteralPath "$Prefix-scan.json" -Encoding utf8
    if ($LASTEXITCODE -ne 0) { throw 'No se pudo capturar el contexto' }
}
finally {
    foreach ($handle in $handles) {
        [void][GhcSnapshotThreads]::ResumeThread($handle)
        [void][GhcSnapshotThreads]::CloseHandle($handle)
    }
}
Get-FileHash -Algorithm SHA256 -LiteralPath $destination |
    Select-Object @{Name='MotorPid'; Expression={$MotorPid}}, Path, Hash
