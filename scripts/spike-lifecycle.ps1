<#
.SYNOPSIS
    Mide el ciclo de vida de VRes: arranque en la bandeja, apertura y cierre
    de la ventana, y salida.

.DESCRIPTION
    Responde la pregunta 1 de docs/hardware/RESULTADOS.md: ¿WebView2 libera
    sus procesos cuando VRes destruye la ventana principal?

    Abre y cierra la ventana de VRes dos veces (se ve en pantalla unos
    segundos) e imprime un JSON con las mediciones.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\spike-lifecycle.ps1 -Exe "$env:LOCALAPPDATA\vres-dev\target\debug\vres.exe"
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Exe,
    [int]$SettleSeconds = 4,
    [int]$ReleaseTimeoutSeconds = 15
)

$ErrorActionPreference = 'Stop'
$identifier = 'io.github.gxnza48.vres'
$processName = [IO.Path]::GetFileNameWithoutExtension($Exe)

Add-Type -Namespace VRes -Name Native -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
'@

function Get-WebViewProcesses {
    @(Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" |
        Where-Object { $_.CommandLine -like "*$identifier*" })
}

function Get-AppProcess {
    Get-Process -Name $processName -ErrorAction SilentlyContinue | Select-Object -First 1
}

function Measure-State([string]$Step) {
    $app = Get-AppProcess
    $webviews = Get-WebViewProcesses
    $webviewMb = 0
    foreach ($webview in $webviews) {
        $p = Get-Process -Id $webview.ProcessId -ErrorAction SilentlyContinue
        if ($p) { $webviewMb += $p.WorkingSet64 / 1MB }
    }
    [ordered]@{
        step             = $Step
        appAlive         = [bool]$app
        windowVisible    = [bool]($app -and $app.MainWindowHandle -ne 0)
        appWorkingSetMb  = if ($app) { [math]::Round($app.WorkingSet64 / 1MB, 1) } else { 0 }
        webviewProcesses = $webviews.Count
        webviewMb        = [math]::Round($webviewMb, 1)
    }
}

function Wait-WebViewsGone([int]$TimeoutSeconds) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.Elapsed.TotalSeconds -lt $TimeoutSeconds) {
        if ((Get-WebViewProcesses).Count -eq 0) { return [math]::Round($watch.Elapsed.TotalSeconds, 2) }
        Start-Sleep -Milliseconds 200
    }
    return $null
}

function Open-Window {
    # Una segunda instancia le pide a la primera que abra la ventana y termina.
    Start-Process -FilePath $Exe -WindowStyle Hidden | Out-Null
    Start-Sleep -Seconds $SettleSeconds
}

function Close-Window {
    $app = Get-AppProcess
    if ($app -and $app.MainWindowHandle -ne 0) {
        [VRes.Native]::PostMessageW($app.MainWindowHandle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    }
}

# Partir de cero.
& $Exe --shutdown | Out-Null
Start-Sleep -Seconds 1

$results = [ordered]@{ exe = $Exe; steps = @() }

# 1. Arranque con Windows: solo bandeja, sin ventana ni WebView2.
Start-Process -FilePath $Exe -ArgumentList '--autostart' -WindowStyle Hidden | Out-Null
Start-Sleep -Seconds $SettleSeconds
$results.steps += Measure-State 'autostart (solo bandeja)'

# 2 y 3. Dos ciclos de abrir y cerrar la ventana.
foreach ($cycle in 1, 2) {
    Open-Window
    $results.steps += Measure-State "ventana abierta ($cycle)"
    Close-Window
    $released = Wait-WebViewsGone $ReleaseTimeoutSeconds
    $state = Measure-State "ventana cerrada ($cycle)"
    $state.webviewReleasedAfterSeconds = $released
    $results.steps += $state
}

# 4. Salida pedida desde otro proceso (lo que usa el instalador).
$watch = [Diagnostics.Stopwatch]::StartNew()
& $Exe --shutdown | Out-Null
$results.shutdownSeconds = [math]::Round($watch.Elapsed.TotalSeconds, 2)
Start-Sleep -Milliseconds 500
$results.appAliveAfterShutdown = [bool](Get-AppProcess)
$results.webviewsAfterShutdown = (Get-WebViewProcesses).Count

$results | ConvertTo-Json -Depth 4
