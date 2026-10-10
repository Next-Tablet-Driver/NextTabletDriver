<#
.SYNOPSIS
  Starts the real app on Windows and checks that it comes up as a GUI application.

.DESCRIPTION
  The WebDriver smoke test (frontend/e2e-native/smoke.ts) cannot run on the Windows CI runners:
  the WebView2 runtime there ignores the remote-debugging switch that msedgedriver passes. This
  script checks what can be checked without it, on the binary that is built in CI:
    - the process keeps running after startup;
    - a visible top-level window appears, titled NextTabletDriver, and it is not a console window;
    - the app did not start a console host (no conhost.exe / OpenConsole.exe child);
    - the WebView2 runtime was started for it.
  The PE subsystem of the binary is checked separately by assert-gui-subsystem.ps1.

.PARAMETER App
  Path of the executable.

.PARAMETER TimeoutSeconds
  How long to wait for the window to appear.
#>
param(
    [string]$App = 'src-tauri/target/debug/app.exe',
    [int]$TimeoutSeconds = 90
)

$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class NativeWindow {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr hWnd, StringBuilder name, int max);
}
'@

$path = (Resolve-Path $App).Path
Write-Host "Starting $path"
$process = Start-Process -FilePath $path -PassThru
$failures = @()

try {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $handle = [IntPtr]::Zero
    while ((Get-Date) -lt $deadline) {
        if ($process.HasExited) { break }
        $process.Refresh()
        if ($process.MainWindowHandle -ne [IntPtr]::Zero) { $handle = $process.MainWindowHandle; break }
        Start-Sleep -Milliseconds 500
    }

    if ($process.HasExited) {
        $failures += "the app exited with code $($process.ExitCode) before showing a window"
    }
    elseif ($handle -eq [IntPtr]::Zero) {
        $failures += "no window appeared within $TimeoutSeconds seconds"
    }
    else {
        $title = $process.MainWindowTitle
        $class = New-Object System.Text.StringBuilder 256
        [void][NativeWindow]::GetClassName($handle, $class, $class.Capacity)
        Write-Host "Window: '$title' (class $class)"
        if ($title -notmatch 'NextTabletDriver') { $failures += "unexpected window title '$title'" }
        if ($class.ToString() -eq 'ConsoleWindowClass') { $failures += 'the main window is a console window' }
    }

    # Let the app settle, then check it is still running and what it started.
    Start-Sleep -Seconds 5
    if ($process.HasExited) { $failures += "the app exited with code $($process.ExitCode) shortly after startup" }

    $children = Get-CimInstance Win32_Process -Filter "ParentProcessId=$($process.Id)" | Select-Object -ExpandProperty Name
    Write-Host "Child processes: $($children -join ', ')"
    if ($children | Where-Object { $_ -in 'conhost.exe', 'OpenConsole.exe', 'cmd.exe' }) {
        $failures += 'the app started a console host'
    }
    if (-not (Get-Process msedgewebview2 -ErrorAction SilentlyContinue)) {
        $failures += 'the WebView2 runtime was not started'
    }
}
finally {
    # The app may outlive its window (close to tray): stop the whole tree.
    & taskkill /pid $process.Id /T /F | Out-Null
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ -ErrorAction Continue }
    exit 1
}
Write-Host 'The app starts as a GUI application: window shown, no console, WebView2 running.'
