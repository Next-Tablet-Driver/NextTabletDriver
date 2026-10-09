<#
.SYNOPSIS
    Fails when a Windows executable is a console program (it would open a terminal on launch).
.PARAMETER Path
    The .exe to inspect, e.g. src-tauri\target\debug\app.exe.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$Path
)

$ErrorActionPreference = "Stop"

$bytes = [System.IO.File]::ReadAllBytes((Resolve-Path $Path))
# e_lfanew -> "PE\0\0" (4) + COFF header (20) -> optional header, `Subsystem` at offset 68.
$pe = [System.BitConverter]::ToInt32($bytes, 0x3c)
$subsystem = [System.BitConverter]::ToUInt16($bytes, $pe + 24 + 68)

switch ($subsystem) {
    2 { Write-Host "OK: $Path is a GUI program (subsystem 2), no console window." }
    3 { throw "$Path is a console program (subsystem 3): it opens a terminal window on launch. Check #![windows_subsystem = `"windows`"] in src-tauri/src/main.rs." }
    default { throw "$Path has an unexpected subsystem: $subsystem" }
}
