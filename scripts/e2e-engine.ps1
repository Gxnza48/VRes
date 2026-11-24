<#
.SYNOPSIS
    Prueba de punta a punta de la red de seguridad del motor de VRes.

.DESCRIPTION
    1. Arranca VRes (sin ventana) con un look guardado y verifica que se aplica.
    2. Mata el proceso a la fuerza: la matriz desaparece sola, las rampas no.
    3. Lo vuelve a abrir: debe restaurar las rampas originales y arrancar en
       modo seguro (color apagado).
    4. Prueba `vres.exe --reset` (restablecer con la app abierta).
    5. Prueba `vres.exe --shutdown` (salida limpia).

    Cambia los colores de la pantalla unos segundos (saturación 140 % y
    gamma 1,10) y al final deja todo como estaba, incluida la sesión guardada.
    Imprime un JSON con el resultado de cada paso.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\e2e-engine.ps1
#>
[CmdletBinding()]
param(
    [string]$Exe = (Join-Path $env:LOCALAPPDATA 'vres-dev\target\debug\vres.exe'),
    [string]$Cli = (Join-Path $env:LOCALAPPDATA 'vres-dev\target\debug\vres-cli.exe'),
    [int]$SettleSeconds = 3
)

$ErrorActionPreference = 'Stop'
$stateDir = Join-Path $env:LOCALAPPDATA 'io.github.gxnza48.vres\state'
$sessionPath = Join-Path $stateDir 'session.json'
$restorePath = Join-Path $stateDir 'restore.json'
$processName = [IO.Path]::GetFileNameWithoutExtension($Exe)

function Invoke-Cli([string[]]$Arguments) {
    $output = & $Cli @Arguments --json
    return ($output | Out-String | ConvertFrom-Json)
}

function Get-RampsAreIdentity {
    $ramps = (Invoke-Cli @('ramp', 'get')).ramps
    return [bool](@($ramps | Where-Object { -not $_.ramp.identity }).Count -eq 0)
}

function Get-MatrixIsIdentity {
    return [bool](Invoke-Cli @('mag', 'get')).identity
}

function Get-RestoreDirty {
    if (-not (Test-Path $restorePath)) { return $false }
    return [bool](Get-Content $restorePath -Raw | ConvertFrom-Json).dirty
}

function Get-SessionCleanExit {
    return [bool](Get-Content $sessionPath -Raw | ConvertFrom-Json).cleanExit
}

function Write-Session([bool]$CleanExit) {
    $session = @{
        schemaVersion = 1
        cleanExit     = $CleanExit
        requested     = @{
            colorEnabled = $true
            look         = @{ saturation = 140; tone = @{ gamma = 1.1 } }
        }
    }
    New-Item -ItemType Directory -Force -Path $stateDir | Out-Null
    $json = $session | ConvertTo-Json -Depth 6
    [IO.File]::WriteAllText($sessionPath, $json, (New-Object Text.UTF8Encoding($false)))
}

function Start-VRes {
    Start-Process -FilePath $Exe -ArgumentList '--autostart' -WindowStyle Hidden | Out-Null
    Start-Sleep -Seconds $SettleSeconds
}

function Test-Running {
    return [bool](Get-Process -Name $processName -ErrorAction SilentlyContinue)
}

# Partir de cero y guardar la sesión del usuario para devolverla al final.
& $Exe --shutdown | Out-Null
Start-Sleep -Milliseconds 500
$backup = if (Test-Path $sessionPath) { Get-Content $sessionPath -Raw } else { $null }

$result = [ordered]@{}
try {
    # 1. Arranque con un look guardado.
    Write-Session -CleanExit $true
    Start-VRes
    $result.started = Test-Running
    $result.respondsToPing = [bool](Invoke-Cli @('status')).responding
    $result.matrixApplied = -not (Get-MatrixIsIdentity)
    $result.rampsApplied = -not (Get-RampsAreIdentity)
    $result.restoreDirtyWhileActive = Get-RestoreDirty

    # 2. Cierre forzado.
    Stop-Process -Name $processName -Force
    Start-Sleep -Seconds 1
    $result.matrixGoneAfterKill = Get-MatrixIsIdentity
    $result.rampsStillModifiedAfterKill = -not (Get-RampsAreIdentity)
    $result.restoreDirtyAfterKill = Get-RestoreDirty

    # 3. Reapertura: recupera y arranca en modo seguro.
    Start-VRes
    $result.rampsRecoveredOnRestart = Get-RampsAreIdentity
    $result.matrixOffInSafeMode = Get-MatrixIsIdentity
    $result.restoreCleanAfterRecovery = -not (Get-RestoreDirty)
    & $Exe --shutdown | Out-Null
    Start-Sleep -Milliseconds 500

    # 4. Restablecer con la app abierta.
    Write-Session -CleanExit $true
    Start-VRes
    $activeBeforeReset = (-not (Get-MatrixIsIdentity)) -and (-not (Get-RampsAreIdentity))
    & $Exe --reset | Out-Null
    Start-Sleep -Seconds 1
    $result.resetRestoresEverything = $activeBeforeReset -and (Get-MatrixIsIdentity) -and (Get-RampsAreIdentity)
    $result.stillRunningAfterReset = Test-Running

    # 5. Salida limpia.
    $watch = [Diagnostics.Stopwatch]::StartNew()
    & $Exe --shutdown | Out-Null
    $result.shutdownSeconds = [math]::Round($watch.Elapsed.TotalSeconds, 2)
    Start-Sleep -Milliseconds 300
    $result.exitedAfterShutdown = -not (Test-Running)
    $result.sessionCleanExit = Get-SessionCleanExit
    $result.restoreCleanAfterShutdown = -not (Get-RestoreDirty)
    $result.displayUntouchedAtEnd = (Get-MatrixIsIdentity) -and (Get-RampsAreIdentity)
}
finally {
    if (Test-Running) { & $Exe --shutdown | Out-Null }
    & $Cli reset | Out-Null
    if ($null -ne $backup) {
        [IO.File]::WriteAllText($sessionPath, $backup, (New-Object Text.UTF8Encoding($false)))
    } elseif (Test-Path $sessionPath) {
        Remove-Item $sessionPath
    }
}

$checks = @($result.Keys | Where-Object { $result[$_] -is [bool] })
$result.ok = @($checks | Where-Object {
    $expected = $true
    $result[$_] -ne $expected
}).Count -eq 0
$result | ConvertTo-Json
