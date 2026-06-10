$tabletsDir = Join-Path $PSScriptRoot "..\tablets"
$outputFile = Join-Path $PSScriptRoot "99-nexttabletdriver.rules"

$rules = @()
$rules += "# NextTabletDriver udev rules"
$rules += "# Allows non-root users in the `"input`" group to access /dev/uinput"
$rules += "# and prevents double input by telling libinput to ignore the original tablets."
$rules += "#"
$rules += "# Installation:"
$rules += "#   sudo cp 99-nexttabletdriver.rules /etc/udev/rules.d/"
$rules += "#   sudo udevadm control --reload-rules"
$rules += "#   sudo udevadm trigger"
$rules += "#"
$rules += "# Then add your user to the input group:"
$rules += "#   sudo usermod -aG input `$USER"
$rules += "#"
$rules += "# You will need to log out and back in for group changes to take effect."
$rules += ""
$rules += "# Grant read/write access to /dev/uinput for the `"input`" group"
$rules += "KERNEL==`"uinput`", SUBSYSTEM==`"misc`", MODE=`"0660`", GROUP=`"input`", TAG+=`"uaccess`""
$rules += ""
$rules += "# Remove virtual tablet joypad devices (prevents tablet acting as a controller in games)"
$rules += "KERNEL==`"js[0-9]*`", SUBSYSTEM==`"input`", ATTRS{name}==`"NextTabletDriver Virtual Pen`", RUN+=`"/usr/bin/env rm %E{DEVNAME}`""
$rules += "KERNEL==`"js[0-9]*`", SUBSYSTEM==`"input`", ATTRS{name}==`"NextTabletDriver Virtual Mouse`", RUN+=`"/usr/bin/env rm %E{DEVNAME}`""
$rules += ""

$vids = @{}
$ignoreRules = @()
$seenCache = @{}

foreach ($file in Get-ChildItem -Path $tabletsDir -Filter "*.json" -Recurse) {
    try {
        $json = Get-Content $file.FullName -Raw | ConvertFrom-Json
        $name = $json.Name
        $libinput = 0
        if ($json.Attributes -and $json.Attributes.libinputoverride) {
            $libinput = [int]$json.Attributes.libinputoverride
        }
        
        if (-not $json.DigitizerIdentifiers) {
            continue
        }
        
        foreach ($id in $json.DigitizerIdentifiers) {
            $vidHex = '{0:x4}' -f $id.VendorID
            $pidHex = '{0:x4}' -f $id.ProductID
            
            $vids[$vidHex] = $true
            
            $cacheKey = "$vidHex-$pidHex"
            if (-not $seenCache[$cacheKey] -and $libinput -gt 0) {
                $seenCache[$cacheKey] = $true
                $ignoreRules += "# $name"
                $ignoreRules += "SUBSYSTEM==`"input`", ATTRS{idVendor}==`"$vidHex`", ATTRS{idProduct}==`"$pidHex`", ENV{LIBINPUT_IGNORE_DEVICE}=`"$libinput`""
            }
        }
    } catch {
        Write-Warning "Failed to parse $($file.FullName)"
    }
}

$rules += "# Grant read access to tablet HID devices via hidraw"
foreach ($vid in $vids.Keys) {
    $rules += "SUBSYSTEM==`"hidraw`", ATTRS{idVendor}==`"$vid`", MODE=`"0660`", GROUP=`"input`""
}

$rules += ""
$rules += "# Prevent double input by ignoring the raw devices in libinput/Wayland/X11"
$rules += $ignoreRules

$rules | Out-File -FilePath $outputFile -Encoding ASCII
Write-Host "Generated $outputFile"