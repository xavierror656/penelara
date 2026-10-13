param(
    [Parameter(Mandatory = $true)][int]$MotorPid,
    [Parameter(Mandatory = $true)][string]$Prefix,
    [int]$PauseMs = 15000
)

$ErrorActionPreference = 'Stop'
$motor = Get-Process -Id $MotorPid -ErrorAction Stop
if ($motor.ProcessName -ne 'MotHor') { throw "PID $MotorPid no es MotHor" }
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path.TrimEnd('\') + '\'
$motorPath = $motor.Path
if (-not $motorPath.StartsWith((Join-Path $workspace '.runtime-replay\core\run-'),
        [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "MotHor debe pertenecer a una corrida aislada dentro de $workspace"
}
$prefixPath = [System.IO.Path]::GetFullPath($Prefix)
if (-not $prefixPath.StartsWith($workspace, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "La captura debe quedar dentro de $workspace"
}
$tracer = Join-Path $workspace '.runtime-replay\Trace-GapReturn.exe'
if (-not (Test-Path -LiteralPath $tracer)) {
    throw "Falta $tracer; compile tools\Trace-GapReturn.c como x86"
}
$tracePath = "$prefixPath-trace.csv"
$logPath = "$prefixPath-trace.log"
foreach ($path in @($tracePath, $logPath, "$prefixPath-scan.json", "$prefixPath-records.json")) {
    if (Test-Path -LiteralPath $path) { throw "La captura ya existe: $path" }
}

$process = Start-Process -FilePath $tracer -ArgumentList @($MotorPid, 30, 1, $PauseMs) `
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
    if ($process.HasExited) { throw "El trazador terminó antes de congelar una llamada: $log" }
} while ([DateTime]::UtcNow -lt $deadline)
if ($log -notmatch 'captura congelada') { throw 'No se detectó una llamada antes del plazo' }

& python (Join-Path $PSScriptRoot 'inspect_native_scoring_records.py') $MotorPid `
    --dump-prefix $prefixPath | Set-Content -LiteralPath "$prefixPath-records.json" -Encoding utf8
if ($LASTEXITCODE -ne 0) { throw 'No se pudieron capturar los registros de puntuación' }
& python (Join-Path $PSScriptRoot 'scan_scoring_object.py') $MotorPid `
    --dump-prefix $prefixPath | Set-Content -LiteralPath "$prefixPath-scan.json" -Encoding utf8
if ($LASTEXITCODE -ne 0) { throw 'No se pudo capturar el contexto' }

$trace = Import-Csv -LiteralPath $tracePath | Select-Object -First 1
$scan = Get-Content -LiteralPath "$prefixPath-scan.json" -Raw | ConvertFrom-Json
$contexts = @($scan.contexts | Where-Object { $_.contextDump })
if ($contexts.Count -ne 1 -or $contexts[0].base -ine "0x$($trace.context)") {
    throw "El contexto congelado no coincide con la llamada: retorno=$($trace.context), escáner=$($contexts.base -join ',')"
}
$process.WaitForExit($PauseMs + 10000) | Out-Null
$process.Refresh()
$traceLog = Get-Content -LiteralPath $logPath -Raw
$exitCode = $process.ExitCode
if (-not $process.HasExited -or ($null -ne $exitCode -and $exitCode -ne 0) -or
    $traceLog -notmatch '(?m)^eventos=1\s*$') {
    throw "El trazador no terminó limpiamente: exited=$($process.HasExited) code=$exitCode log=$traceLog"
}
$result = & python (Join-Path $PSScriptRoot 'analyze_native_gap_triangles.py') $prefixPath |
    ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'No se pudo reproducir el retorno' }
$row = [string]$trace.row
$predictedReturn = $result.perTeacherPredictedReturn.$row
$predictedCount = $result.perTeacherDailyCountIncrement.$row
$predictedSpanExcess = $result.perTeacherSpanExcess.$row
if ($null -eq $predictedReturn -or $null -eq $predictedCount -or
    $null -eq $predictedSpanExcess) {
    throw "La fila $row no está cubierta por la reproducción del barrido"
}
if ([int]$trace.eax -ne [int]$predictedReturn -or
    [int]$trace.stack30 -ne [int]$predictedCount -or
    [int]$trace.stack34 -ne [int]$predictedSpanExcess) {
    throw "Diferencia en fila ${row}: eax=$($trace.eax)/$predictedReturn, contador=$($trace.stack30)/$predictedCount, exceso=$($trace.stack34)/$predictedSpanExcess"
}
[pscustomobject]@{
    motorPid = $MotorPid
    teacherRow = [int]$trace.row
    nativeReturn = [int]$trace.eax
    predictedReturn = [int]$predictedReturn
    nativeDailyCount = [int]$trace.stack30
    predictedDailyCount = [int]$predictedCount
    nativeSpanExcess = [int]$trace.stack34
    predictedSpanExcess = [int]$predictedSpanExcess
    context = $trace.context
    trace = $tracePath
} | ConvertTo-Json -Compress
