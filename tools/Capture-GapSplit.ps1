param(
    [Parameter(Mandatory = $true)][int]$MotorPid,
    [Parameter(Mandatory = $true)][string]$Prefix,
    [int]$PauseMs = 15000,
    [int]$TargetRow = -1
)

$ErrorActionPreference = 'Stop'
$motor = Get-Process -Id $MotorPid -ErrorAction Stop
if ($motor.ProcessName -ne 'MotHor') { throw "PID $MotorPid no es MotHor" }
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path.TrimEnd('\') + '\'
if (-not $motor.Path.StartsWith((Join-Path $workspace '.runtime-replay\core\run-'),
        [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'MotHor debe pertenecer a una corrida aislada'
}
$prefixPath = [System.IO.Path]::GetFullPath($Prefix)
if (-not $prefixPath.StartsWith($workspace, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'La captura debe quedar dentro del workspace'
}
$tracer = Join-Path $workspace '.runtime-replay\Trace-GapSplit.exe'
if (-not (Test-Path -LiteralPath $tracer)) {
    throw "Falta $tracer; compile tools\Trace-GapSplit.c como x86"
}
$tracePath = "$prefixPath-trace.csv"
$logPath = "$prefixPath-trace.log"
foreach ($path in @($tracePath, $logPath, "$prefixPath-scan.json", "$prefixPath-records.json")) {
    if (Test-Path -LiteralPath $path) { throw "La captura ya existe: $path" }
}

$process = Start-Process -FilePath $tracer -ArgumentList @($MotorPid, 30, $PauseMs, $TargetRow) `
    -WorkingDirectory $workspace -WindowStyle Hidden -PassThru `
    -RedirectStandardOutput $tracePath -RedirectStandardError $logPath
$deadline = [DateTime]::UtcNow.AddSeconds(20)
do {
    Start-Sleep -Milliseconds 100
    $process.Refresh()
    $log = if (Test-Path -LiteralPath $logPath) {
        Get-Content -LiteralPath $logPath -Raw -ErrorAction SilentlyContinue
    } else { '' }
    if ($log -match 'captura congelada') { break }
    if ($process.HasExited) { throw "El trazador terminó antes de la primera llamada: $log" }
} while ([DateTime]::UtcNow -lt $deadline)
if ($log -notmatch 'captura congelada') { throw 'No se detectó la primera llamada' }

& python (Join-Path $PSScriptRoot 'inspect_native_scoring_records.py') $MotorPid `
    --dump-prefix $prefixPath | Set-Content -LiteralPath "$prefixPath-records.json" -Encoding utf8
if ($LASTEXITCODE -ne 0) { throw 'No se pudieron capturar los registros' }
& python (Join-Path $PSScriptRoot 'scan_scoring_object.py') $MotorPid `
    --dump-prefix $prefixPath | Set-Content -LiteralPath "$prefixPath-scan.json" -Encoding utf8
if ($LASTEXITCODE -ne 0) { throw 'No se pudo capturar el contexto' }

$first = Import-Csv -LiteralPath $tracePath | Select-Object -First 1
if ($null -eq $first -or [int]$first.phase -ne 1) { throw 'Falta la primera traza' }
$scan = Get-Content -LiteralPath "$prefixPath-scan.json" -Raw | ConvertFrom-Json
$contexts = @($scan.contexts | Where-Object { $_.contextDump })
if ($contexts.Count -ne 1 -or $contexts[0].base -ine "0x$($first.context)") {
    throw "El contexto congelado no coincide: retorno=$($first.context), escáner=$($contexts.base -join ',')"
}
$process.WaitForExit($PauseMs + 10000) | Out-Null
$process.Refresh()
$traceLog = Get-Content -LiteralPath $logPath -Raw
$exitCode = $process.ExitCode
if (-not $process.HasExited -or ($null -ne $exitCode -and $exitCode -ne 0) -or
    $traceLog -notmatch '(?m)^eventos=2\s*$') {
    throw "El trazador no terminó limpiamente: exited=$($process.HasExited) code=$exitCode log=$traceLog"
}
$events = @(Import-Csv -LiteralPath $tracePath)
if ($events.Count -ne 2 -or [int]$events[1].phase -ne 2 -or
    $events[0].row -ne $events[1].row -or
    $events[0].thread -ne $events[1].thread -or
    $events[0].context -ne $events[1].context) {
    throw 'Las dos llamadas no corresponden a la misma fila, hilo y contexto'
}
$result = & python (Join-Path $PSScriptRoot 'analyze_native_gap_triangles.py') $prefixPath |
    ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or -not $result.splitAtNoon) {
    throw 'El estado capturado no reproduce la ruta de dos llamadas'
}
$row = [string]$first.row
$predicted = @($result.perTeacherCalls.$row)
if ($predicted.Count -ne 2) { throw "Fila $row sin dos llamadas predichas" }
if ([int]$events[0].eax -ne [int]$predicted[0].return -or
    [int]$events[1].eax -ne [int]$predicted[1].return -or
    [int]$events[0].stack30 -ne [int]$predicted[0].dailyCount -or
    [int]$events[1].stack30 -ne [int]$result.perTeacherDailyCountIncrement.$row -or
    [int]$events[0].stack34 -ne [int]$predicted[0].spanExcess -or
    [int]$events[1].stack34 -ne [int]$result.perTeacherSpanExcess.$row) {
    throw "Diferencia en fila ${row}: trazas=$($events | ConvertTo-Json -Compress), predicción=$($predicted | ConvertTo-Json -Compress)"
}
[pscustomobject]@{
    motorPid = $MotorPid
    teacherRow = [int]$first.row
    nativeReturns = @([int]$events[0].eax, [int]$events[1].eax)
    predictedReturns = @([int]$predicted[0].return, [int]$predicted[1].return)
    nativeDailyCounts = @([int]$events[0].stack30, [int]$events[1].stack30)
    predictedDailyCounts = @([int]$predicted[0].dailyCount, [int]$result.perTeacherDailyCountIncrement.$row)
    nativeSpanExcesses = @([int]$events[0].stack34, [int]$events[1].stack34)
    predictedSpanExcesses = @([int]$predicted[0].spanExcess, [int]$result.perTeacherSpanExcess.$row)
    context = $first.context
    trace = $tracePath
} | ConvertTo-Json -Compress
