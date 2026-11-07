# msictl — MSI Laptop Hardware & Power Controller

> **Fast, lightweight, native Rust utility for MSI laptops under Linux (fan curves, performance profiles, Cooler Boost, discrete GPU status, battery charge limits).**

Designed and tested on **MSI PS63 Modern 8RD (MS-16S1)** and compatible MSI laptops featuring the `msi-ec` platform driver.

---

## ✨ Features

- 🤫 **Performance & Fan Profiles:** Switch between `Silent` (quiet curves, Eco shift), `Balanced` (dynamic auto fans), and `Turbo` (high clocks).
- ❄️ **Cooler Boost Support:** One-key toggle for 100% maximum dual-fan cooling.
- 📊 **Real-time Telemetry:** Direct EC readout of CPU/GPU fan speeds (%), temperatures, battery charge, and hardware limits.
- 🔍 **Dynamic Hardware Auto-Discovery:** Automatically detects NVIDIA GPU PCI address (via vendor ID `0x10de` and VGA/3D class) and primary ACPI battery (BAT0, BAT1, etc.) without hardcoding bus paths.
- 🔋 **Safe GPU Power Tracking:** Tracks NVIDIA discrete GPU performance state (`P8` low power idle) without unstable bus hacks.
- 🖥️ **Desktop OSD & Hyprland Hotkeys:** Instant desktop notifications via `notify-send` on mode switch.
- 🪟 **Modern Slint GUI + System Tray:** Native Wayland/X11 GUI with live telemetry dashboard, battery threshold switcher, Cooler Boost toggle, and background System Tray indicator (StatusNotifierItem/AppIndicator).
- ⚡ **Zero-Sudo Udev Integration:** Direct sysfs access for group `wheel` without entering root passwords.

---

## 🚀 Installation

### 1. Build & Install Binaries
```bash
cargo build --release
sudo install -m 755 target/release/msictl /usr/local/bin/msictl
sudo install -m 755 target/release/msictl-gui /usr/local/bin/msictl-gui
sudo install -m 644 assets/msictl.desktop /usr/share/applications/msictl.desktop
```

### 2. Configure Udev (Run without sudo)
A ready-to-use udev rule file is provided in `assets/70-msi-ec.rules`:
```bash
sudo cp assets/70-msi-ec.rules /etc/udev/rules.d/70-msi-ec.rules
sudo udevadm control --reload && sudo udevadm trigger
```
Ensure your user belongs to the `wheel` group.

---

## 🎨 GUI & Tray Usage

Launch the GUI dashboard and background tray:
```bash
msictl-gui
```
- **Tray Icon**: Displays current profile icon in the system panel (Waybar, Quickshell, KDE, GNOME).
- **Left-Click Tray**: Toggles the Control Center dashboard window.
- **Right-Click Tray Menu**: Quick switcher between Silent / Balanced / Turbo, Cooler Boost toggle, Webcam killswitch, and Show/Quit.

---

## ⌨️ CLI Usage

---

## ⌨️ CLI Usage

### System Status Card:
```bash
msictl status
```

### Switch Performance Modes:
```bash
msictl mode silent      # Quiet fans, Eco CPU clocks
msictl mode balanced    # Dynamic fan curve, balanced performance
msictl mode turbo       # High performance profile
```

### Cycle Modes (Hotkeys):
```bash
msictl cycle            # Silent -> Balanced -> Turbo -> Silent
```

### Cooler Boost:
```bash
msictl boost toggle     # Toggle 100% maximum fan force
msictl boost on
msictl boost off
```

### GPU Power Status:
```bash
msictl gpu              # Query NVIDIA dGPU power and P-State
```

### Battery Charge Limit:
```bash
msictl battery 80       # Limit max charge to 80% (preserve battery longevity)
```

---

## 🪟 Hyprland Hotkey Integration

Add to `~/.config/hypr/bindings.lua`:
```lua
-- MSI Laptop Hardware & Cooling Control
o.bind("SUPER + F7", "MSI Profile Cycle", "msictl cycle")
o.bind("SUPER + SHIFT + F7", "Cooler Boost Toggle", "msictl boost")
```

---

## 📜 License

GPL-3.0-or-later.
