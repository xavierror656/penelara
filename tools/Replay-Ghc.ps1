param(
    [string]$Run = 'probe',
    [string]$InputName = 'ejemploDosMarcosHorarios.xghc',
    [switch]$SkipGenerate,
    [int]$Seconds = 12,
    [int]$EngineCommand = 0,
    [int]$PlannerCommand = 0,
    [int]$EditorCommand = 0,
    [switch]$EngineClose,
    [switch]$AcceptEngineClose,
    [switch]$ListMenus,
    [switch]$ListEditorControls,
    [switch]$ListEditorMenus,
    [switch]$ListEngineControls,
    [switch]$SampleEngineControls,
    [switch]$SampleEngineMemory,
    [switch]$SaveCandidates,
    [switch]$Isolated,
    [string]$PlannerOverride = '',
    [string]$MotorOverride = '',
    [double]$SuspendPlannerAt = -1,
    [double]$ResumePlannerAt = -1,
    [int]$SampleMs = 1000,
    [int]$CommandAt = 4
)

$ErrorActionPreference = 'Stop'
if ($SampleMs -lt 20) { throw 'SampleMs debe ser al menos 20' }
if ($PlannerOverride -and -not $Isolated) { throw 'PlannerOverride requiere Isolated' }
if ($MotorOverride -and -not $Isolated) { throw 'MotorOverride requiere Isolated' }
$seedRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\.runtime-replay\core')).Path
$root = $seedRoot
if ($Isolated) {
    $root = Join-Path $seedRoot "run-$Run"
    if (Test-Path -LiteralPath $root) { throw "La corrida aislada ya existe: $root" }
    New-Item -ItemType Directory -Path $root | Out-Null
    foreach ($name in @('PlaniHor.exe', 'MotHor.exe', 'EditHor.exe', 'ghcMain.exe', $InputName)) {
        Copy-Item -LiteralPath (Join-Path $seedRoot $name) -Destination (Join-Path $root $name)
    }
    if ($PlannerOverride) {
        Copy-Item -LiteralPath (Resolve-Path -LiteralPath $PlannerOverride).Path -Destination (Join-Path $root 'PlaniHor.exe') -Force
    }
    if ($MotorOverride) {
        Copy-Item -LiteralPath (Resolve-Path -LiteralPath $MotorOverride).Path -Destination (Join-Path $root 'MotHor.exe') -Force
    }
}
$tempDir = Join-Path $root "temp-$Run"
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
$project = Join-Path $root $InputName
$projectStem = [System.IO.Path]::GetFileNameWithoutExtension($InputName)
$motorPath = Join-Path $root 'MotHor.exe'
$editorPath = Join-Path $root 'EditHor.exe'
$mainPath = Join-Path $root 'ghcMain.exe'
$registrationPath = Join-Path $root 'regGHC.exe'
$resultIndex = 1
do {
    $result = Join-Path $root ('{0}R{1}.xrho' -f $projectStem, $resultIndex)
    $resultIndex++
} while (Test-Path -LiteralPath $result)
$motorFile = Join-Path $tempDir 'auxghcompM0'
$oldTemp = $env:TEMP
$oldTmp = $env:TMP
$beforeMotors = @(Get-Process MotHor -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
$beforeEditors = @(Get-Process EditHor -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
$beforeMain = @(Get-Process ghcMain -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
$beforeRegistration = @(Get-Process regGHC -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)

Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class GhcReplayWin32 {
    public delegate bool EnumWindowCallback(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowCallback callback, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern bool EnumChildWindows(IntPtr parent, EnumWindowCallback callback, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);
    [DllImport("user32.dll", SetLastError=true)]
    public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern IntPtr GetMenu(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern int GetMenuItemCount(IntPtr hMenu);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern int GetMenuString(IntPtr hMenu, uint item, StringBuilder text, int maxCount, uint flags);
    [DllImport("user32.dll")]
    public static extern IntPtr GetSubMenu(IntPtr hMenu, int position);
    [DllImport("user32.dll")]
    public static extern uint GetMenuItemID(IntPtr hMenu, int position);
    [DllImport("user32.dll")]
    public static extern int GetDlgCtrlID(IntPtr hWnd);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern int GetClassName(IntPtr hWnd, StringBuilder text, int maxCount);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern IntPtr OpenProcess(uint access, bool inheritHandle, uint processId);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern bool ReadProcessMemory(IntPtr process, IntPtr address, byte[] buffer, int size, out IntPtr bytesRead);
    [DllImport("kernel32.dll")]
    public static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern IntPtr OpenThread(uint access, bool inheritHandle, uint threadId);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern uint SuspendThread(IntPtr thread);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern uint ResumeThread(IntPtr thread);
}
'@

function Get-ProcessWindow([int]$processId, [string]$titlePattern) {
    $callback = [GhcReplayWin32+EnumWindowCallback]{
        param($hWnd, $unused)
        [uint32]$owner = 0
        [void][GhcReplayWin32]::GetWindowThreadProcessId($hWnd, [ref]$owner)
        if ($owner -eq $processId) {
            $title = [System.Text.StringBuilder]::new(512)
            [void][GhcReplayWin32]::GetWindowText($hWnd, $title, $title.Capacity)
            if ($title.ToString() -like $titlePattern) {
                $script:foundWindow = $hWnd
                $script:foundTitle = $title.ToString()
                return $false
            }
        }
        return $true
    }
    $script:foundWindow = [IntPtr]::Zero
    $script:foundTitle = ''
    [void][GhcReplayWin32]::EnumWindows($callback, [IntPtr]::Zero)
    return $script:foundWindow
}

function Get-WindowTitles([int]$processId) {
    $script:matchingTitles = @()
    $callback = [GhcReplayWin32+EnumWindowCallback]{
        param($hWnd, $unused)
        [uint32]$owner = 0
        [void][GhcReplayWin32]::GetWindowThreadProcessId($hWnd, [ref]$owner)
        if ($owner -eq $processId) {
            $title = [System.Text.StringBuilder]::new(512)
            [void][GhcReplayWin32]::GetWindowText($hWnd, $title, $title.Capacity)
            if ($title.Length -gt 0) { $script:matchingTitles += $title.ToString() }
        }
        return $true
    }
    [void][GhcReplayWin32]::EnumWindows($callback, [IntPtr]::Zero)
    return $script:matchingTitles
}

function Get-ChildTitles([IntPtr]$window) {
    $script:childTitles = @()
    $callback = [GhcReplayWin32+EnumWindowCallback]{
        param($hWnd, $unused)
        $title = [System.Text.StringBuilder]::new(512)
        [void][GhcReplayWin32]::GetWindowText($hWnd, $title, $title.Capacity)
        if ($title.Length -gt 0) { $script:childTitles += $title.ToString() }
        return $true
    }
    [void][GhcReplayWin32]::EnumChildWindows($window, $callback, [IntPtr]::Zero)
    return $script:childTitles
}

function Get-ChildControls([IntPtr]$window) {
    $script:childControls = @()
    $callback = [GhcReplayWin32+EnumWindowCallback]{
        param($hWnd, $unused)
        $title = [System.Text.StringBuilder]::new(512)
        $class = [System.Text.StringBuilder]::new(128)
        [void][GhcReplayWin32]::GetWindowText($hWnd, $title, $title.Capacity)
        [void][GhcReplayWin32]::GetClassName($hWnd, $class, $class.Capacity)
        $script:childControls += @{ id = [GhcReplayWin32]::GetDlgCtrlID($hWnd); class = $class.ToString(); text = $title.ToString() }
        return $true
    }
    [void][GhcReplayWin32]::EnumChildWindows($window, $callback, [IntPtr]::Zero)
    return $script:childControls
}

function Get-FileState([string]$path, [double]$sampleSecond = 0, [string]$candidateLabel = '') {
    if (-not (Test-Path -LiteralPath $path)) { return $null }
    try {
        $stream = [System.IO.File]::Open($path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite -bor [System.IO.FileShare]::Delete)
        try {
            $buffer = [System.IO.MemoryStream]::new()
            try {
                $stream.CopyTo($buffer)
                $bytes = $buffer.ToArray()
            } finally { $buffer.Dispose() }
        } finally { $stream.Dispose() }
    } catch [System.IO.IOException] { return $null }
    $digest = [System.Security.Cryptography.SHA256]::Create()
    try { $hash = [BitConverter]::ToString($digest.ComputeHash($bytes)).Replace('-', '') }
    finally { $digest.Dispose() }
    if ($SaveCandidates -and $candidateLabel -and $script:savedHashes[$candidateLabel] -ne $hash) {
        $snapshot = Join-Path $tempDir ('candidate-{0}-{1:N3}-{2}.bin' -f $candidateLabel, $sampleSecond, $hash.Substring(0, 8))
        [System.IO.File]::WriteAllBytes($snapshot, $bytes)
        $script:savedHashes[$candidateLabel] = $hash
    }
    return @{ bytes = $bytes.Length; modifiedUtc = (Get-Item -LiteralPath $path).LastWriteTimeUtc.ToString('o'); sha256 = $hash }
}

function Get-EngineMemory([int]$processId) {
    $handle = [GhcReplayWin32]::OpenProcess(0x410, $false, [uint32]$processId)
    if ($handle -eq [IntPtr]::Zero) { return $null }
    try {
        $values = @{}
        foreach ($item in @(@('incompleteWeight', 0x493230), @('unplacedCount', 0x493234), @('bestIncompleteWeight', 0x493240), @('bestCompleteWeight', 0x49324c), @('scoreSlot0', 0x493250), @('scoreSlot1', 0x493254))) {
            $bytes = New-Object byte[] 4
            $read = [IntPtr]::Zero
            if ([GhcReplayWin32]::ReadProcessMemory($handle, [IntPtr]$item[1], $bytes, 4, [ref]$read) -and $read.ToInt32() -eq 4) {
                $values[$item[0]] = [BitConverter]::ToUInt32($bytes, 0)
            }
        }
        $pointer = New-Object byte[] 4
        $read = [IntPtr]::Zero
        if ([GhcReplayWin32]::ReadProcessMemory($handle, [IntPtr]0x4932c0, $pointer, 4, [ref]$read) -and $read.ToInt32() -eq 4) {
            $app = [BitConverter]::ToUInt32($pointer, 0)
            if ($app -ne 0 -and [GhcReplayWin32]::ReadProcessMemory($handle, [IntPtr]([long]$app + 0xdc), $pointer, 4, [ref]$read) -and $read.ToInt32() -eq 4) {
                $data = [BitConverter]::ToUInt32($pointer, 0)
                if ($data -ne 0) {
                    $counters = New-Object byte[] 0x30
                    if ([GhcReplayWin32]::ReadProcessMemory($handle, [IntPtr]([long]$data + 0xd0640), $counters, 0x30, [ref]$read) -and $read.ToInt32() -eq 0x30) {
                        $values['resultCounters'] = @{
                            studentGaps = [BitConverter]::ToUInt32($counters, 0x08)
                            studentNonpreferred = [BitConverter]::ToUInt32($counters, 0x18)
                            teacherGaps = [BitConverter]::ToUInt32($counters, 0x0c)
                            teacherNonpreferred = [BitConverter]::ToUInt32($counters, 0x1c)
                            teacherExtraPresence = [BitConverter]::ToUInt32($counters, 0x10)
                            subjectNonpreferred = [BitConverter]::ToUInt32($counters, 0x20)
                            lessonNonpreferred = [BitConverter]::ToUInt32($counters, 0x24)
                            roomNonpreferred = [BitConverter]::ToUInt32($counters, 0x14)
                            teacherDailyImbalance = [BitConverter]::ToUInt16($counters, 0x2a)
                        }
                    }
                }
            }
        }
        return $values
    } finally {
        [void][GhcReplayWin32]::CloseHandle($handle)
    }
}

function Get-MenuRows([IntPtr]$menu, [string]$prefix = '') {
    $rows = @()
    $count = [GhcReplayWin32]::GetMenuItemCount($menu)
    for ($position = 0; $position -lt $count; $position++) {
        $caption = [System.Text.StringBuilder]::new(256)
        [void][GhcReplayWin32]::GetMenuString($menu, [uint32]$position, $caption, $caption.Capacity, 0x400)
        $sub = [GhcReplayWin32]::GetSubMenu($menu, $position)
        $name = $prefix + $caption.ToString().Replace('&', '')
        if ($sub -ne [IntPtr]::Zero) {
            $rows += Get-MenuRows $sub ($name + ' / ')
        } elseif ($name.Trim()) {
            $rows += @{ name = $name; id = [GhcReplayWin32]::GetMenuItemID($menu, $position) }
        }
    }
    return $rows
}

$planner = $null
$savedHashes = @{}
$suspendedPlannerThreads = @()
$plannerWasSuspended = $false
try {
    $env:TEMP = $tempDir
    $env:TMP = $tempDir
    $planner = Start-Process -FilePath (Join-Path $root 'PlaniHor.exe') -ArgumentList ('"' + $project + '"') -WorkingDirectory $root -WindowStyle Hidden -PassThru
    $window = [IntPtr]::Zero
    for ($attempt = 0; $attempt -lt 30; $attempt++) {
        Start-Sleep -Milliseconds 500
        $planner.Refresh()
        if ($SkipGenerate -and $planner.HasExited) { break }
        $window = Get-ProcessWindow $planner.Id '*Planificador de horarios*'
        if ($window -ne [IntPtr]::Zero) { break }
    }
    $dispatchedToEditor = $false
    if ($window -eq [IntPtr]::Zero -and $SkipGenerate -and $planner.HasExited -and $planner.ExitCode -eq 0) {
        for ($attempt = 0; $attempt -lt 10; $attempt++) {
            $dispatchedToEditor = @(Get-Process EditHor -ErrorAction SilentlyContinue | Where-Object { $beforeEditors -notcontains $_.Id -and $_.Path -eq $editorPath }).Count -gt 0
            if ($dispatchedToEditor) { break }
            Start-Sleep -Milliseconds 200
        }
    }
    if ($window -eq [IntPtr]::Zero -and -not $dispatchedToEditor) {
        $titles = @(Get-WindowTitles $planner.Id) -join ' | '
        $helpers = @(Get-Process regGHC -ErrorAction SilentlyContinue | Where-Object { $beforeRegistration -notcontains $_.Id -and $_.Path -eq $registrationPath } | Select-Object -ExpandProperty Id) -join ','
        $newEditors = @(Get-Process EditHor -ErrorAction SilentlyContinue | Where-Object { $beforeEditors -notcontains $_.Id } | Select-Object -ExpandProperty Id) -join ','
        $planner.Refresh()
        $exitState = $(if ($planner.HasExited) { "salió con código $($planner.ExitCode)" } else { 'sigue en ejecución' })
        throw "PlaniHor no abrió ventana principal; estado=[$exitState]; ventanas=[$titles]; regGHC=[$helpers]; EditHor=[$newEditors]"
    }
    $pre = @{ stage = 'before'; plannerPid = $planner.Id; title = $script:foundTitle; dispatchedToEditor = $dispatchedToEditor; result = (Get-FileState $result); motorFile = (Get-FileState $motorFile) }
    $pre | ConvertTo-Json -Compress -Depth 5
    if ($ListMenus) {
        @{ stage = 'menus-before'; rows = @(Get-MenuRows ([GhcReplayWin32]::GetMenu($window))) } | ConvertTo-Json -Compress -Depth 5
    }
    if (-not $SkipGenerate) {
        if (-not [GhcReplayWin32]::PostMessage($window, 0x111, [IntPtr]32863, [IntPtr]::Zero)) {
            throw 'No se pudo activar Generar horario'
        }
    }
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    $previousSecond = 0.0
    while ($timer.Elapsed.TotalSeconds -lt $Seconds) {
        Start-Sleep -Milliseconds $SampleMs
        $second = [Math]::Round($timer.Elapsed.TotalSeconds, 3)
        $planner.Refresh()
        if ($SuspendPlannerAt -ge 0 -and -not $plannerWasSuspended -and $previousSecond -lt $SuspendPlannerAt -and $second -ge $SuspendPlannerAt) {
            foreach ($thread in $planner.Threads) {
                $handle = [GhcReplayWin32]::OpenThread(0x2, $false, [uint32]$thread.Id)
                if ($handle -ne [IntPtr]::Zero) {
                    if ([GhcReplayWin32]::SuspendThread($handle) -ne [uint32]::MaxValue) {
                        $suspendedPlannerThreads += $handle
                    } else { [void][GhcReplayWin32]::CloseHandle($handle) }
                }
            }
            $plannerWasSuspended = $true
        }
        if ($ResumePlannerAt -ge 0 -and $plannerWasSuspended -and $previousSecond -lt $ResumePlannerAt -and $second -ge $ResumePlannerAt) {
            foreach ($handle in $suspendedPlannerThreads) {
                [void][GhcReplayWin32]::ResumeThread($handle)
                [void][GhcReplayWin32]::CloseHandle($handle)
            }
            $suspendedPlannerThreads = @()
        }
        $motors = @(Get-Process MotHor -ErrorAction SilentlyContinue | Where-Object { $beforeMotors -notcontains $_.Id -and $_.Path -eq $motorPath })
        $commandSent = $false
        if (($EngineCommand -ne 0 -or $EngineClose) -and $previousSecond -lt $CommandAt -and $second -ge $CommandAt -and $motors.Count -eq 1) {
            $engineWindow = Get-ProcessWindow $motors[0].Id '*GHC-Engine*'
            if ($engineWindow -eq [IntPtr]::Zero) { throw 'No se encontró la ventana de GHC-Engine' }
            if ($EngineClose) {
                $commandSent = [GhcReplayWin32]::PostMessage($engineWindow, 0x10, [IntPtr]::Zero, [IntPtr]::Zero)
            } else {
                $commandSent = [GhcReplayWin32]::PostMessage($engineWindow, 0x111, [IntPtr]$EngineCommand, [IntPtr]::Zero)
            }
        }
        if ($AcceptEngineClose -and $previousSecond -lt ($CommandAt + 1) -and $second -ge ($CommandAt + 1) -and $motors.Count -eq 1) {
            $modalWindow = Get-ProcessWindow $motors[0].Id '*Motor de horarios GHC*'
            if ($modalWindow -eq [IntPtr]::Zero) { throw 'No se encontró la confirmación de cierre del motor' }
            $commandSent = [GhcReplayWin32]::PostMessage($modalWindow, 0x111, [IntPtr]1, [IntPtr]::Zero)
        }
        if ($PlannerCommand -ne 0 -and $previousSecond -lt $CommandAt -and $second -ge $CommandAt) {
            $commandSent = [GhcReplayWin32]::PostMessage($window, 0x111, [IntPtr]$PlannerCommand, [IntPtr]::Zero)
        }
        $editors = @(Get-Process EditHor -ErrorAction SilentlyContinue | Where-Object { $beforeEditors -notcontains $_.Id -and $_.Path -eq $editorPath })
        if ($EditorCommand -ne 0 -and $previousSecond -lt $CommandAt -and $second -ge $CommandAt -and $editors.Count -eq 1) {
            $resultWindow = Get-ProcessWindow $editors[0].Id '*EditHor - *.xrho*'
            if ($resultWindow -ne [IntPtr]::Zero) {
                $commandSent = [GhcReplayWin32]::PostMessage($resultWindow, 0x111, [IntPtr]$EditorCommand, [IntPtr]::Zero)
            }
        }
        $mains = @(Get-Process ghcMain -ErrorAction SilentlyContinue | Where-Object { $beforeMain -notcontains $_.Id -and $_.Path -eq $mainPath })
        $registration = @(Get-Process regGHC -ErrorAction SilentlyContinue | Where-Object { $beforeRegistration -notcontains $_.Id -and $_.Path -eq $registrationPath })
        $editorDetails = @($editors | ForEach-Object {
            $info = Get-CimInstance Win32_Process -Filter "ProcessId=$($_.Id)" -ErrorAction SilentlyContinue
            $editorWindow = Get-ProcessWindow $_.Id '*EditHor*'
            $resultWindow = Get-ProcessWindow $_.Id '*EditHor - *.xrho*'
            $reportWindow = Get-ProcessWindow $_.Id '*Informes sobre el resultado*'
            @{ pid = $_.Id; path = $_.Path; commandLine = $info.CommandLine; windows = @(Get-WindowTitles $_.Id); children = $(if ($ListEditorControls -and $editorWindow -ne [IntPtr]::Zero) { @(Get-ChildTitles $editorWindow) } else { @() }); resultWindowChildren = $(if ($ListEditorControls -and $resultWindow -ne [IntPtr]::Zero) { @(Get-ChildTitles $resultWindow) } else { @() }); reportControls = $(if ($ListEditorControls -and $reportWindow -ne [IntPtr]::Zero) { @(Get-ChildControls $reportWindow) } else { @() }); menus = $(if ($ListEditorMenus -and $resultWindow -ne [IntPtr]::Zero) { @(Get-MenuRows ([GhcReplayWin32]::GetMenu($resultWindow))) } else { @() }) }
        })
        $engineDetails = @($motors | ForEach-Object {
            if ($ListEngineControls -or $SampleEngineControls) {
                $engineWindow = Get-ProcessWindow $_.Id '*GHC-Engine*'
                $modalWindow = Get-ProcessWindow $_.Id '*Motor de horarios GHC*'
                @{ pid = $_.Id; windows = $(if ($ListEngineControls) { @(Get-WindowTitles $_.Id) } else { @() }); children = $(if ($ListEngineControls -and $engineWindow -ne [IntPtr]::Zero) { @(Get-ChildControls $engineWindow) } elseif ($SampleEngineControls -and $engineWindow -ne [IntPtr]::Zero) { @(Get-ChildControls $engineWindow | Where-Object { $_.text -match '^\s*-?\d+([.,]\d+)?\s*$' }) } else { @() }); modalChildren = $(if ($ListEngineControls -and $modalWindow -ne [IntPtr]::Zero) { @(Get-ChildTitles $modalWindow) } else { @() }) }
            }
        })
        $resultState = Get-FileState $result $second 'result'
        $motorState = Get-FileState $motorFile $second 'motor'
        $memory = $(if ($SampleEngineMemory -and $motors.Count -eq 1) { Get-EngineMemory $motors[0].Id } else { $null })
        $row = @{ stage = 'running'; second = $second; plannerAlive = (-not $planner.HasExited); plannerSuspendedThreads = $suspendedPlannerThreads.Count; plannerTitle = $planner.MainWindowTitle; motorPids = @($motors | Select-Object -ExpandProperty Id); mainPids = @($mains | Select-Object -ExpandProperty Id); registrationPids = @($registration | Select-Object -ExpandProperty Id); engineDetails = $engineDetails; engineMemory = $memory; editors = $editorDetails; result = $resultState; motorFile = $motorState; commandSent = $commandSent }
        if ($ListMenus) {
            $row.plannerWindows = @(Get-WindowTitles $planner.Id)
            $row.plannerChildren = @(Get-ChildTitles $window)
        }
        $row | ConvertTo-Json -Compress -Depth 5
        if ($ListMenus -and $previousSecond -lt $CommandAt -and $second -ge $CommandAt) {
            @{ stage = 'menus-running'; rows = @(Get-MenuRows ([GhcReplayWin32]::GetMenu($window))) } | ConvertTo-Json -Compress -Depth 5
        }
        $previousSecond = $second
    }
}
finally {
    foreach ($handle in $suspendedPlannerThreads) {
        [void][GhcReplayWin32]::ResumeThread($handle)
        [void][GhcReplayWin32]::CloseHandle($handle)
    }
    if ($null -ne $planner -and -not $planner.HasExited) { Stop-Process -Id $planner.Id -Force -ErrorAction SilentlyContinue }
    Get-Process MotHor -ErrorAction SilentlyContinue | Where-Object { $beforeMotors -notcontains $_.Id -and $_.Path -eq $motorPath } | Stop-Process -Force -ErrorAction SilentlyContinue
    Get-Process EditHor -ErrorAction SilentlyContinue | Where-Object { $beforeEditors -notcontains $_.Id -and $_.Path -eq $editorPath } | Stop-Process -Force -ErrorAction SilentlyContinue
    Get-Process ghcMain -ErrorAction SilentlyContinue | Where-Object { $beforeMain -notcontains $_.Id -and $_.Path -eq $mainPath } | Stop-Process -Force -ErrorAction SilentlyContinue
    Get-Process regGHC -ErrorAction SilentlyContinue | Where-Object { $beforeRegistration -notcontains $_.Id -and $_.Path -eq $registrationPath } | Stop-Process -Force -ErrorAction SilentlyContinue
    $env:TEMP = $oldTemp
    $env:TMP = $oldTmp
}
