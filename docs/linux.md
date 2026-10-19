# Linux setup

NextTabletDriver works on Linux with X11, Wayland and XWayland. It reads your tablet directly and creates virtual input devices, so no extra layer is needed.

## What the driver needs

Two things need permission on Linux:

1. **Your tablet.** The driver reads it through a `hidraw` device file (`/dev/hidraw*`).
2. **`/dev/uinput`.** The driver uses it to create the virtual pen and mouse. It is a feature of the Linux kernel, and the `uinput` kernel module must be loaded.

The file `scripts/99-nexttabletdriver.rules` gives your user both permissions. It is a *udev rules* file. udev is the Linux service that creates device files and sets who can use them. The rules also tell the system to ignore the real tablet, so the cursor does not move twice.

## Install

### Debian and Ubuntu (.deb)

Download the `.deb` from the [releases page](https://github.com/Next-Tablet-Driver/NextTabletDriver/releases) and install it.

```bash
sudo apt install ./NextTabletDriver_*.deb
```

The package installs the udev rules. Reload them once, or unplug and replug your tablet:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### AppImage

The releases page has an AppImage for x86_64 and one for ARM64. An AppImage is a single file that runs without installing. On many systems it needs FUSE 2 (the package is often called `libfuse2`).

```bash
chmod +x NextTabletDriver_*.AppImage
./NextTabletDriver_*.AppImage
```

The AppImage does not install the udev rules, so do it yourself:

```bash
curl -fsSL -o 99-nexttabletdriver.rules \
  https://raw.githubusercontent.com/Next-Tablet-Driver/NextTabletDriver/main/scripts/99-nexttabletdriver.rules
sudo cp 99-nexttabletdriver.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

With an AppImage, **Run at startup** may not work. The AppImage runs from a temporary folder, and the startup entry points to that folder. Use the `.deb`, or add the AppImage to your desktop's startup applications yourself.

### Arch Linux

A package recipe is in [`packaging/linux/PKGBUILD`](../packaging/linux/PKGBUILD). It builds the latest code from git, and installs the program, the icon, the license and the udev rules. It is a work in progress and it is not on the AUR yet. If it fails, use the `.deb` or the AppImage instead.

```bash
curl -fsSLO https://raw.githubusercontent.com/Next-Tablet-Driver/NextTabletDriver/main/packaging/linux/PKGBUILD
makepkg -si
```

### NixOS

The repository is a flake with a NixOS module. One switch installs the udev rules, loads the `uinput` module and starts the driver as a user service.

```nix
{
  inputs.nexttabletdriver.url = "github:Next-Tablet-Driver/NextTabletDriver";

  outputs = { nixpkgs, nexttabletdriver, ... }: {
    nixosConfigurations.<your-hostname> = nixpkgs.lib.nixosSystem {
      modules = [
        nexttabletdriver.nixosModules.default
        {
          services.nexttabletdriver.enable = true;
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

Options of `services.nexttabletdriver`:

| Option | What it does |
| --- | --- |
| `enable` | Turns the module on. |
| `package` | Which package to use. The flake's default package is used if you do not set it. |
| `user` | Adds this user to the `input` group. Only needed when you have no `systemd-logind` session (see below). |
| `extraArgs` | Extra command-line arguments for the service, for example `[ "--autostart" ]` to start without showing the window. |

If you use Home Manager and want a per-user service, import `homeManagerModules.default` and set `services.nexttabletdriver.enable = true;`. Home Manager cannot install udev rules or load kernel modules, so you still need the NixOS module, or you must set them up yourself.

## Permissions without systemd-logind

On a normal desktop, the udev rules give your user access as soon as you sign in. This works through `systemd-logind`. You do not need to do anything else.

Some setups have no logind session, for example a headless machine or a very small window manager. Use the `input` group instead, then sign out and back in:

```bash
sudo usermod -aG input "$USER"
```

Check with `groups "$USER"`.

## Check that it works

Start the driver with your tablet plugged in. Then list the input devices:

```bash
cat /proc/bus/input/devices
```

You should see **NextTabletDriver Virtual Pen** and **NextTabletDriver Virtual Mouse**. To watch the events as they happen:

```bash
sudo libinput debug-events
```

## Troubleshooting

### Permission denied when the driver starts

- Install the udev rules and reload them (see above).
- Check that you are in a `systemd-logind` desktop session: run `loginctl` and look for your session. If not, use the `input` group.
- Check that the `uinput` module is loaded (next item).

### The uinput module is not loaded

```bash
sudo modprobe uinput
```

To load it at every boot:

```bash
echo uinput | sudo tee /etc/modules-load.d/uinput.conf
```

### The cursor moves twice, or the real tablet also moves it

The udev rules tell libinput to ignore the physical tablet. If your tablet is new to the rules file, add it (see below), reinstall the rules, and replug the tablet.

### More help

See the general [Troubleshooting](troubleshooting.md) page.

## Using a tablet that is not in the rules file

The rules file is built from the tablet definitions. If you add your own definition in a `tablets` folder (see [Adding a tablet](adding-a-tablet.md#try-a-definition-on-your-own-computer)), build the rules again so that Linux gives access to your tablet and ignores it:

```bash
scripts/generate_udev_rules.sh        # needs jq
```

Or, with PowerShell:

```bash
pwsh scripts/generate_udev_rules.ps1
```

Then install the new `scripts/99-nexttabletdriver.rules` file as shown in the AppImage section.
