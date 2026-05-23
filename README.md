# NextTabletDriver (NextTD)

NextTabletDriver is a high-performance, cross-platform tablet driver designed for artists and rhythm game players. It provides a modern alternative to vendor-specific drivers, focusing on low latency, hardware compatibility, and a streamlined user experience.

## Key Features

- **High-Frequency Polling**: Optimized for 1000Hz+ reporting rates to ensure minimal input delay.
- **Native Linux Support**: Uses the kernel's `uinput` interface for seamless compatibility with X11 and Wayland.
- **Advanced Mapping**: Precise control over active area, rotation, and screen projection with sub-pixel accuracy.
- **Modular Filter System**: Includes industry-standard smoothing filters (like Devocub Antichatter) to eliminate sensor noise.
- **Cross-Platform**: Consistent experience across Windows and Linux.
- **WebSocket Integration**: Real-time telemetry for streaming overlays and external tools.

## Supported Hardware

NextTabletDriver supports a wide range of tablets from various manufacturers, including:

- **Wacom** (Intuos, Bamboo, One, etc.)
- **Huion** (Kamvas, Inspiroy, etc.)
- **Gaomon**
- **XP-Pen**
- **VEIKK**
- **Artisul**
- ...and many others via community-contributed JSON configurations.

## Installation

### Windows
1. Download the latest release.
2. Run the installer or extract the portable version.
3. Launch `next_tablet_driver.exe`.

### Linux
1. Ensure the `uinput` kernel module is loaded.
2. Install the provided udev rules to grant permission to your user:
   ```bash
   sudo cp scripts/99-nexttabletdriver.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules && sudo udevadm trigger
   sudo usermod -aG input $USER
   ```
3. Log out and back in.
4. Run the driver executable.

## Documentation for Developers

The core logic of NextTabletDriver is written in Rust. You can generate the technical documentation by running:

```bash
cargo doc --no-deps --open
```

## Contributing

We welcome contributions! Whether it's adding support for a new tablet model, fixing a bug, or suggesting a feature, please feel free to open an issue or submit a pull request on GitHub.

## License

This project is licensed under the MIT License see the [LICENSE](LICENSE) file for details.
