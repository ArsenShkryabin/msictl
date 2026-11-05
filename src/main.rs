use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

const MSI_EC_PATH: &str = "/sys/devices/platform/msi-ec";
const BAT_PATH: &str = "/sys/class/power_supply/BAT1";
const GPU_PCI_PATH: &str = "/sys/bus/pci/devices/0000:02:00.0";
const GPU_PARENT_PCI_PATH: &str = "/sys/bus/pci/devices/0000:00:1c.4";

// ANSI Styling
const C_RESET: &str = "\x1b[0m";
const C_BOLD: &str = "\x1b[1m";
const C_RED: &str = "\x1b[1;31m";
const C_GREEN: &str = "\x1b[1;32m";
const C_YELLOW: &str = "\x1b[1;33m";
const C_BLUE: &str = "\x1b[1;34m";
const C_MAGENTA: &str = "\x1b[1;35m";
const C_CYAN: &str = "\x1b[1;36m";
const C_DIM: &str = "\x1b[2m";

#[derive(Parser)]
#[command(name = "msictl")]
#[command(author = "Arsen Shkryabin")]
#[command(version = "0.1.0")]
#[command(about = "MSI Laptop Hardware & Power Controller (Fan curves, Cooler Boost, GPU D3cold, Profiles)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show comprehensive system status (fans, temps, battery, GPU, profile)
    Status,
    /// Set performance and fan profile (silent, balanced, turbo, eco)
    Mode {
        /// Target mode: silent | balanced | turbo | eco
        mode: String,
    },
    /// Cycle through modes: silent -> balanced -> turbo -> silent (ideal for hotkeys)
    Cycle,
    /// Control Cooler Boost (100% maximum fan speed)
    Boost {
        /// Action: toggle | on | off
        action: Option<String>,
    },
    /// Manage discrete NVIDIA GPU power state (D3cold suspension)
    Gpu {
        /// Action: status | power-save (suspend GPU) | performance (wake GPU)
        action: Option<String>,
    },
    /// Manage battery charging threshold
    Battery {
        /// Optional threshold percentage (e.g. 80, 100)
        threshold: Option<u8>,
    },
    /// Toggle hardware webcam killswitch via EC
    Cam {
        /// Action: toggle | on | off
        action: Option<String>,
    },
    /// Live real-time terminal monitoring dashboard
    Watch,
}

fn read_sysfs(path: &str) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn write_sysfs(path: &str, value: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("File does not exist: {path}"));
    }
    match fs::write(path, value) {
        Ok(_) => Ok(()),
        Err(e) => {
            // Attempt via sudo fallback if permission denied
            let status = Command::new("sudo")
                .args(["sh", "-c", &format!("echo '{}' > '{}'", value, path)])
                .status()
                .map_err(|se| format!("Failed to run sudo: {se}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("Permission denied writing to {path} ({e})"))
            }
        }
    }
}

fn notify_osd(title: &str, body: &str, icon: &str) {
    let _ = Command::new("notify-send")
        .args([
            "-a", "MSI Control",
            "-u", "low",
            "-i", icon,
            title,
            body,
        ])
        .spawn();
}

fn get_gpu_status() -> (&'static str, &'static str) {
    if Path::new(GPU_PCI_PATH).exists() {
        let status = read_sysfs(&format!("{GPU_PCI_PATH}/power/runtime_status"))
            .unwrap_or_else(|| "active".into());
        if status == "suspended" {
            ("D3hot (Suspended)", C_YELLOW)
        } else {
            ("D0 (Active)", C_GREEN)
        }
    } else {
        let parent_status = read_sysfs(&format!("{GPU_PARENT_PCI_PATH}/power/runtime_status"))
            .unwrap_or_default();
        if parent_status == "suspended" {
            ("D3cold (Powered Off - Power Saving)", C_CYAN)
        } else {
            ("Offline", C_DIM)
        }
    }
}

fn show_status() {
    let fw = read_sysfs(&format!("{MSI_EC_PATH}/fw_version")).unwrap_or_else(|| "N/A".into());
    let fan_mode = read_sysfs(&format!("{MSI_EC_PATH}/fan_mode")).unwrap_or_else(|| "auto".into());
    let shift_mode = read_sysfs(&format!("{MSI_EC_PATH}/shift_mode")).unwrap_or_else(|| "auto".into());
    let cooler_boost = read_sysfs(&format!("{MSI_EC_PATH}/cooler_boost")).unwrap_or_else(|| "off".into());

    let cpu_temp: i32 = read_sysfs(&format!("{MSI_EC_PATH}/cpu/realtime_temperature"))
        .and_then(|s| s.parse().ok()).unwrap_or(0);
    let cpu_fan: i32 = read_sysfs(&format!("{MSI_EC_PATH}/cpu/realtime_fan_speed"))
        .and_then(|s| s.parse().ok()).unwrap_or(0);

    let gpu_temp: i32 = read_sysfs(&format!("{MSI_EC_PATH}/gpu/realtime_temperature"))
        .and_then(|s| s.parse().ok()).unwrap_or(0);
    let gpu_fan: i32 = read_sysfs(&format!("{MSI_EC_PATH}/gpu/realtime_fan_speed"))
        .and_then(|s| s.parse().ok()).unwrap_or(0);

    let bat_cap = read_sysfs(&format!("{BAT_PATH}/capacity")).unwrap_or_else(|| "?".into());
    let bat_stat = read_sysfs(&format!("{BAT_PATH}/status")).unwrap_or_else(|| "?".into());
    let bat_thresh = read_sysfs(&format!("{BAT_PATH}/charge_control_end_threshold")).unwrap_or_else(|| "100".into());

    let (gpu_state, gpu_color) = get_gpu_status();

    println!();
    println!("{C_BOLD}{C_CYAN}╔══════════════════════════════════════════════════════════════════╗{C_RESET}");
    println!("{C_BOLD}{C_CYAN}║                    MSI Laptop Control (msictl)                   ║{C_RESET}");
    println!("{C_BOLD}{C_CYAN}╚══════════════════════════════════════════════════════════════════╝{C_RESET}");
    println!("{C_DIM} EC Firmware: {fw} | MSI MS-16S1 Platform{C_RESET}\n");

    // Profile Box
    println!("{C_BOLD}► Performance & Cooling Profile:{C_RESET}");
    let mode_str = match (fan_mode.as_str(), shift_mode.as_str(), cooler_boost.as_str()) {
        (_, _, "on") => format!("{C_RED}❄️ Cooler Boost (100% Fan Force){C_RESET}"),
        ("silent", _, _) => format!("{C_GREEN}🤫 Silent Mode (Eco Shift + Quiet Fans){C_RESET}"),
        ("advanced", "turbo", _) | (_, "turbo", _) => format!("{C_MAGENTA}🚀 Turbo Mode (Max Performance){C_RESET}"),
        _ => format!("{C_BLUE}⚖️ Balanced Mode (Comfort Shift + Auto Fans){C_RESET}"),
    };
    println!("  Active Mode:   {mode_str}");
    println!("  Fan Mode:      {C_BOLD}{fan_mode}{C_RESET} | Shift Mode: {C_BOLD}{shift_mode}{C_RESET}");
    println!();

    // CPU Telemetry
    let cpu_color = if cpu_temp > 75 { C_RED } else if cpu_temp > 60 { C_YELLOW } else { C_GREEN };
    println!("{C_BOLD}► CPU Metrics:{C_RESET}");
    println!("  Temperature:   {cpu_color}{cpu_temp}°C{C_RESET}");
    println!("  Fan Speed:     {C_BOLD}{cpu_fan}%{C_RESET}");
    println!();

    // GPU Telemetry
    let gpu_temp_color = if gpu_temp > 70 { C_RED } else if gpu_temp > 55 { C_YELLOW } else { C_GREEN };
    println!("{C_BOLD}► GPU (NVIDIA GTX 1050 Ti Max-Q):{C_RESET}");
    println!("  Power State:   {gpu_color}{gpu_state}{C_RESET}");
    println!("  Temperature:   {gpu_temp_color}{gpu_temp}°C{C_RESET}");
    println!("  Fan Speed:     {C_BOLD}{gpu_fan}%{C_RESET}");
    println!();

    // Battery Box
    println!("{C_BOLD}► Battery & Power:{C_RESET}");
    println!("  Charge:        {C_BOLD}{bat_cap}%{C_RESET} ({bat_stat})");
    println!("  Health Cap:    {C_CYAN}{bat_thresh}% limit{C_RESET} (Hardware threshold)");
    println!();
}

fn set_mode(mode: &str) {
    match mode.to_lowercase().as_str() {
        "silent" | "quiet" | "eco" => {
            let _ = write_sysfs(&format!("{MSI_EC_PATH}/cooler_boost"), "off");
            let _ = write_sysfs(&format!("{MSI_EC_PATH}/fan_mode"), "silent");
            let _ = write_sysfs(&format!("{MSI_EC_PATH}/shift_mode"), "eco");
            println!("{C_GREEN}✓ Switched to SILENT mode (Eco profile, quiet fan curves).{C_RESET}");
            notify_osd("MSI Profile: Silent", "Eco power profile, quiet fan curves activated", "weather-clear-night");
        }
        "balanced" | "comfort" | "auto" => {
            let _ = write_sysfs(&format!("{MSI_EC_PATH}/cooler_boost"), "off");
            let _ = write_sysfs(&format!("{MSI_EC_PATH}/fan_mode"), "auto");
            let _ = write_sysfs(&format!("{MSI_EC_PATH}/shift_mode"), "comfort");
            println!("{C_BLUE}✓ Switched to BALANCED mode (Comfort profile, auto fan control).{C_RESET}");
            notify_osd("MSI Profile: Balanced", "Comfort profile, dynamic fan control activated", "preferences-system");
        }
        "turbo" | "performance" | "max" => {
            let _ = write_sysfs(&format!("{MSI_EC_PATH}/cooler_boost"), "off");
            let _ = write_sysfs(&format!("{MSI_EC_PATH}/fan_mode"), "advanced");
            let _ = write_sysfs(&format!("{MSI_EC_PATH}/shift_mode"), "turbo");
            println!("{C_MAGENTA}✓ Switched to TURBO mode (Turbo profile, maximum sustained clocks).{C_RESET}");
            notify_osd("MSI Profile: Turbo", "Maximum performance profile activated", "system-run");
        }
        other => {
            eprintln!("{C_RED}Error: Unknown mode '{other}'. Choose: silent | balanced | turbo{C_RESET}");
        }
    }
}

fn cycle_mode() {
    let fan_mode = read_sysfs(&format!("{MSI_EC_PATH}/fan_mode")).unwrap_or_default();
    let shift_mode = read_sysfs(&format!("{MSI_EC_PATH}/shift_mode")).unwrap_or_default();

    if fan_mode == "silent" || shift_mode == "eco" {
        set_mode("balanced");
    } else if shift_mode == "comfort" || fan_mode == "auto" {
        set_mode("turbo");
    } else {
        set_mode("silent");
    }
}

fn set_boost(action: Option<String>) {
    let current = read_sysfs(&format!("{MSI_EC_PATH}/cooler_boost")).unwrap_or_else(|| "off".into());
    let target = match action.as_deref() {
        Some("on") | Some("enable") => "on",
        Some("off") | Some("disable") => "off",
        _ => if current == "on" { "off" } else { "on" },
    };

    if let Err(e) = write_sysfs(&format!("{MSI_EC_PATH}/cooler_boost"), target) {
        eprintln!("{C_RED}Error: {e}{C_RESET}");
        return;
    }

    if target == "on" {
        println!("{C_RED}❄️ Cooler Boost ENABLED (fans forced to 100% RPM).{C_RESET}");
        notify_osd("Cooler Boost: ON", "Dual fans forced to 100% maximum RPM", "weather-snow");
    } else {
        println!("{C_GREEN}❄️ Cooler Boost DISABLED (restored to profile curve).{C_RESET}");
        notify_osd("Cooler Boost: OFF", "Fans restored to profile curve", "weather-clear");
    }
}

fn set_gpu_power(action: Option<String>) {
    match action.as_deref() {
        Some("power-save") | Some("off") | Some("suspend") => {
            // Check if any process is actively using NVIDIA
            let output = Command::new("fuser")
                .arg("/dev/nvidia0")
                .output();
            if let Ok(out) = output {
                if !out.stdout.is_empty() {
                    eprintln!("{C_RED}Cannot suspend GPU: Active processes are using /dev/nvidia0!{C_RESET}");
                    return;
                }
            }

            println!("Powering down NVIDIA discrete GPU into D3cold...");
            let res = Command::new("sudo")
                .args(["sh", "-c", "echo 1 > /sys/bus/pci/devices/0000:02:00.0/remove 2>/dev/null"])
                .status();
            match res {
                Ok(s) if s.success() => {
                    println!("{C_CYAN}✓ NVIDIA GPU powered off (D3cold state). Saving ~3-5 Watts of battery.{C_RESET}");
                    notify_osd("NVIDIA GPU: Powered Off", "PCI port suspended into D3cold (saving 3-5W)", "battery");
                }
                _ => eprintln!("{C_RED}Failed to remove GPU device from PCI bus.{C_RESET}"),
            }
        }
        Some("performance") | Some("on") | Some("wake") => {
            println!("Rescanning PCI bus to wake NVIDIA GPU...");
            let res = Command::new("sudo")
                .args(["sh", "-c", "echo 1 > /sys/bus/pci/rescan"])
                .status();
            match res {
                Ok(s) if s.success() => {
                    sleep(Duration::from_millis(500));
                    println!("{C_GREEN}✓ NVIDIA GPU re-enumerated and ready for prime-run / compute.{C_RESET}");
                    notify_osd("NVIDIA GPU: Active", "GPU awakened and ready for graphics/compute", "video-display");
                }
                _ => eprintln!("{C_RED}Failed to rescan PCI bus.{C_RESET}"),
            }
        }
        _ => {
            let (state, color) = get_gpu_status();
            println!("{C_BOLD}NVIDIA GPU Status:{C_RESET} {color}{state}{C_RESET}");
            println!("{C_DIM}Usage: msictl gpu power-save (suspend to D3cold) | msictl gpu performance (wake){C_RESET}");
        }
    }
}

fn set_battery(thresh: Option<u8>) {
    let path = format!("{BAT_PATH}/charge_control_end_threshold");
    if let Some(t) = thresh {
        if !(50..=100).contains(&t) {
            eprintln!("{C_RED}Error: Threshold must be between 50% and 100%{C_RESET}");
            return;
        }
        if let Err(e) = write_sysfs(&path, &t.to_string()) {
            eprintln!("{C_RED}Error setting threshold: {e}{C_RESET}");
        } else {
            println!("{C_GREEN}✓ Battery charge limit set to {t}%.{C_RESET}");
            notify_osd("Battery Threshold", &format!("Charge limit set to {t}%"), "battery");
        }
    } else {
        let current = read_sysfs(&path).unwrap_or_else(|| "100".into());
        println!("{C_BOLD}Current Battery Charge Limit:{C_RESET} {C_CYAN}{current}%{C_RESET}");
    }
}

fn set_cam(action: Option<String>) {
    let current = read_sysfs(&format!("{MSI_EC_PATH}/webcam_block")).unwrap_or_else(|| "off".into());
    let target = match action.as_deref() {
        Some("block") | Some("off") => "on",
        Some("unblock") | Some("on") => "off",
        _ => if current == "on" { "off" } else { "on" },
    };

    if let Err(e) = write_sysfs(&format!("{MSI_EC_PATH}/webcam_block"), target) {
        eprintln!("{C_RED}Error: {e}{C_RESET}");
        return;
    }

    if target == "on" {
        println!("{C_RED}📷 Webcam BLOCKED in hardware via EC.{C_RESET}");
        notify_osd("Webcam: Blocked", "Camera hardware-isolated by EC", "camera-disabled");
    } else {
        println!("{C_GREEN}📷 Webcam ENABLED.{C_RESET}");
        notify_osd("Webcam: Active", "Camera hardware enabled", "camera-photo");
    }
}

fn run_watch() {
    print!("\x1b[2J"); // Clear screen
    loop {
        print!("\x1b[H"); // Cursor to home
        show_status();
        println!("{C_DIM}Refreshing every 1s. Press Ctrl+C to exit.{C_RESET}");
        sleep(Duration::from_secs(1));
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Status) => show_status(),
        Some(Commands::Mode { mode }) => set_mode(&mode),
        Some(Commands::Cycle) => cycle_mode(),
        Some(Commands::Boost { action }) => set_boost(action),
        Some(Commands::Gpu { action }) => set_gpu_power(action),
        Some(Commands::Battery { threshold }) => set_battery(threshold),
        Some(Commands::Cam { action }) => set_cam(action),
        Some(Commands::Watch) => run_watch(),
    }
}
