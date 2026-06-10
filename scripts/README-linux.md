# NextTabletDriver - Linux Setup Guide

## Prerequisites

NextTabletDriver communicates with your tablet via raw USB (HID) and creates a
virtual input device through the Linux kernel's `uinput` interface.
This works **natively** with X11, Wayland, and XWayland no compatibility layer needed.

## Quick Setup

### 1. Install udev rules (recommended)

```bash
sudo cp scripts/99-nexttabletdriver.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### 2. Add your user to the `input` group

```bash
sudo usermod -aG input $USER
```

> **Log out and back in** for group changes to take effect.

### 3. Run the driver

```bash
./next_tablet_driver
```

---

## NixOS Configuration

NixOS users can add the following to their `configuration.nix` instead of
manually copying udev rules:

```nix
{ pkgs, ... }:

{
  # Install NextTabletDriver udev rules to grant permissions and prevent double input
  services.udev.packages = [
    (pkgs.writeTextFile {
      name = "nexttabletdriver-udev-rules";
      text = builtins.readFile ./scripts/99-nexttabletdriver.rules;
      destination = "/etc/udev/rules.d/99-nexttabletdriver.rules";
    })
  ];

  # Ensure the uinput kernel module is loaded
  boot.kernelModules = [ "uinput" ];

  # Add your user to the input group
  users.users.<your-username>.extraGroups = [ "input" ];
}
```

Then rebuild:

```bash
sudo nixos-rebuild switch
```

---

## Troubleshooting

### "Permission denied" when starting the driver

Make sure:
1. The udev rules are installed and reloaded
2. Your user is in the `input` group (`groups $USER` to check)
3. You have logged out and back in after adding the group

### The uinput module is not loaded

```bash
sudo modprobe uinput
```

To load it automatically on boot, add `uinput` to `/etc/modules-load.d/`:

```bash
echo "uinput" | sudo tee /etc/modules-load.d/uinput.conf
```

### Verifying the virtual device is created

Once the driver is running and a tablet is connected:

```bash
# List all input devices - look for "NextTabletDriver Virtual Pen"
cat /proc/bus/input/devices

# Watch events in real time
sudo libinput debug-events
```

## Adding a Custom Tablet

If you add a custom tablet configuration JSON inside the `tablets/` directory, you will need to regenerate the udev rules to ensure it is properly ignored by Wayland/X11.

You can regenerate the rules by running the included PowerShell script:

```bash
pwsh scripts/generate_udev_rules.ps1
```

Then, reinstall the rules using the steps in the **Quick Setup** section.
