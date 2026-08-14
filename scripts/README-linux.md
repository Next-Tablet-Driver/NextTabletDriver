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

That's it for a normal desktop session: the rules grant access to `/dev/uinput` and
the tablet's `hidraw` device to the currently logged-in user instantly, via
`systemd-logind`'s dynamic ACLs. No group membership or logout is required.

### 2. (Fallback) Add your user to the `input` group

Only needed for headless setups or sessions without `systemd-logind` (some minimal
window manager or SSH-only setups):

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

NixOS users can use the flake's `nixosModules.default`, which installs the udev
rules, loads the `uinput` kernel module, and sets up a `systemd --user` service
to run the driver in the background, all behind a single `enable` flag.

Add the flake as an input and import the module in your `configuration.nix`:

```nix
{
  inputs.nexttabletdriver.url = "github:Next-Tablet-Driver/NextTabletDriver";

  outputs = { nixpkgs, nexttabletdriver, ... }: {
    nixosConfigurations.<your-hostname> = nixpkgs.lib.nixosSystem {
      modules = [
        nexttabletdriver.nixosModules.default
        {
          services.nexttabletdriver.enable = true;

          # Optional fallback: only needed for headless sessions without
          # systemd-logind. Interactive desktop sessions get access
          # instantly via the udev rules' TAG+="uaccess".
          services.nexttabletdriver.user = "<your-username>";
        }
      ];
    };
  };
}
```

Then rebuild:

```bash
sudo nixos-rebuild switch
```

Home-manager users who prefer a per-user autostart instead of a system-wide
service can import `homeManagerModules.default` and set
`services.nexttabletdriver.enable = true;` instead (udev rules and the
`uinput` kernel module still need to come from the NixOS module or be
configured manually, since home-manager cannot install either).

---

## Troubleshooting

### "Permission denied" when starting the driver

Make sure:
1. The udev rules are installed and reloaded
2. You are in a `systemd-logind` desktop session (`loginctl` should list your session);
   otherwise fall back to the `input` group and log out and back in after adding it
   (`groups $USER` to check membership)

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
