# msictl — MSI Laptop Hardware & Power Controller

> **Fast, lightweight, native Rust utility for MSI laptops under Linux (fan curves, performance profiles, Cooler Boost, discrete GPU status, battery charge limits).**

Designed and tested on **MSI PS63 Modern 8RD (MS-16S1)** and compatible MSI laptops featuring the `msi-ec` platform driver.

---

## ✨ Features

- 🤫 **Performance & Fan Profiles:** Switch between `Silent` (quiet curves, Eco shift), `Balanced` (dynamic auto fans), and `Turbo` (high clocks).
- ❄️ **Cooler Boost Support:** One-key toggle for 100% maximum dual-fan cooling.
- 📊 **Real-time Telemetry:** Direct EC readout of CPU/GPU fan speeds (%), temperatures, battery charge, and hardware limits.
- 🔋 **Safe GPU Power Tracking:** Tracks NVIDIA GeForce GTX 1050 Ti Max-Q performance state (`P8` low power idle) without unstable bus hacks.
- 🖥️ **Desktop OSD & Hyprland Hotkeys:** Instant desktop notifications via `notify-send` on mode switch.
- ⚡ **Zero-Sudo Udev Integration:** Direct sysfs access for group `wheel` without entering root passwords.

---

## 🚀 Installation

### 1. Build & Install Binary
```bash
cargo build --release
sudo install -m 755 target/release/msictl /usr/local/bin/msictl
```

### 2. Configure Udev (Run without sudo)
```bash
sudo bash -c 'cat << "EOF" > /etc/udev/rules.d/70-msi-ec.rules
ACTION=="add|change", SUBSYSTEM=="platform", KERNEL=="msi-ec", RUN+="/usr/bin/chmod -R g+w /sys/devices/platform/msi-ec", RUN+="/usr/bin/chgrp -R wheel /sys/devices/platform/msi-ec"
EOF
udevadm control --reload && udevadm trigger -s platform -p /devices/platform/msi-ec
chmod -R g+w /sys/devices/platform/msi-ec && chgrp -R wheel /sys/devices/platform/msi-ec'
```

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
