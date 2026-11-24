<#
.SYNOPSIS
    Prepara el entorno de desarrollo de VRes en esta PC.

.DESCRIPTION
    El repo puede vivir dentro de OneDrive. Para que OneDrive no sincronice (ni
    bloquee) los gigas que genera la compilación de Rust, la carpeta `target` de
    Cargo se mueve a %LOCALAPPDATA%\vres-dev\target.

    El script genera `.cargo/config.local.toml` (ignorado por git), que
    `.cargo/config.toml` incluye automáticamente. Es idempotente: se puede correr
    las veces que haga falta.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\setup-dev.ps1
#>
[CmdletBinding()]
param(
    # Carpeta de compilación. Por defecto, fuera de OneDrive.
    [string]$TargetDir = (Join-Path $env:LOCALAPPDATA 'vres-dev\target')
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$configPath = Join-Path $root '.cargo\config.local.toml'

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $configPath) | Out-Null
New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null

# TOML admite barras normales en rutas de Windows y así no hay que escapar nada.
$tomlPath = $TargetDir -replace '\\', '/'
$content = @"
# Generado por scripts/setup-dev.ps1. No se versiona.
[build]
target-dir = "$tomlPath"
"@

# UTF-8 sin BOM (Set-Content de Windows PowerShell 5.1 agregaría un BOM).
[System.IO.File]::WriteAllText($configPath, $content + "`n", (New-Object System.Text.UTF8Encoding($false)))

Write-Host "Listo: Cargo compila en $TargetDir"
Write-Host ''
Write-Host 'Siguientes pasos:'
Write-Host '  npm ci'
Write-Host '  npm run tauri dev'
