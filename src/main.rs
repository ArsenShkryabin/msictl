use clap::{Parser, Subcommand};
use msictl::{HardwareController, ProfileMode, SystemTelemetry};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

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
#[command(version = "0.2.0")]
#[command(about = "MSI Laptop Hardware & Power Controller (Fan curves, Cooler Boost, GPU D3cold, Profiles)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show comprehensive system status (fans, temps, battery, GPU, profile)
    Status,
    /// Set performance and fan profile (silent, balanced, turbo)
    Mode {
        /// Target mode: silent | balanced | turbo
        mode: String,
    },
    /// Cycle through modes: silent -> balanced -> turbo -> silent (ideal for hotkeys)
    Cycle,
    /// Control Cooler Boost (100% maximum fan speed)
    Boost {
        /// Action: toggle | on | off
        action: Option<String>,
    },
    /// Show discrete NVIDIA GPU power state and P-state metrics
    Gpu,
    /// Manage battery charging threshold
    Battery {
        /// Optional threshold percentage (e.g. 60, 80, 100)
        threshold: Option<u8>,
    },
    /// Toggle hardware webcam killswitch via EC
    Cam {
        /// Action: toggle | on | off
        action: Option<String>,
    },
    /// Live real-time terminal monitoring dashboard
    Watch,
    /// Launch graphical user interface
    Gui,
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

fn print_status(telemetry: &SystemTelemetry, hw: &HardwareController) {
    println!();
    println!("{C_BOLD}{C_CYAN}╔══════════════════════════════════════════════════════════════════╗{C_RESET}");
    println!("{C_BOLD}{C_CYAN}║                    MSI Laptop Control (msictl)                   ║{C_RESET}");
    println!("{C_BOLD}{C_CYAN}╚══════════════════════════════════════════════════════════════════╝{C_RESET}");
    println!("{C_DIM} EC Firmware: {} | Battery: {}{C_RESET}\n",
        telemetry.fw_version,
        hw.get_battery_path().map(|p| p.display().to_string()).unwrap_or_else(|| "Not found".into())
    );

    // Profile Box
    println!("{C_BOLD}► Performance & Cooling Profile:{C_RESET}");
    let mode_str = if telemetry.cooler_boost {
        format!("{C_RED}❄️ Cooler Boost (100% Fan Force){C_RESET}")
    } else if telemetry.fan_mode == "silent" || telemetry.shift_mode == "eco" {
        format!("{C_GREEN}🤫 Silent Mode (Eco Shift + Quiet Fans){C_RESET}")
    } else if telemetry.shift_mode == "turbo" || telemetry.fan_mode == "advanced" {
        format!("{C_MAGENTA}🚀 Turbo Mode (Max Performance){C_RESET}")
    } else {
        format!("{C_BLUE}⚖️ Balanced Mode (Comfort Shift + Auto Fans){C_RESET}")
    };

    println!("  Active Mode:   {mode_str}");
    println!("  Fan Mode:      {C_BOLD}{}{C_RESET} | Shift Mode: {C_BOLD}{}{C_RESET}", telemetry.fan_mode, telemetry.shift_mode);
    println!();

    // CPU Telemetry
    let cpu_color = if telemetry.cpu_temp > 75 { C_RED } else if telemetry.cpu_temp > 60 { C_YELLOW } else { C_GREEN };
    println!("{C_BOLD}► CPU Metrics:{C_RESET}");
    println!("  Temperature:   {cpu_color}{}°C{C_RESET}", telemetry.cpu_temp);
    println!("  Fan Speed:     {C_BOLD}{}%{C_RESET}", telemetry.cpu_fan_speed);
    println!();

    // GPU Telemetry
    let gpu_color = if telemetry.gpu_pstate == "P8" { C_GREEN } else if telemetry.gpu_active { C_YELLOW } else { C_DIM };
    let gpu_temp_color = if telemetry.gpu_temp > 70 { C_RED } else if telemetry.gpu_temp > 55 { C_YELLOW } else { C_GREEN };
    println!("{C_BOLD}► GPU (PCI: {}):{C_RESET}", hw.get_gpu_pci_path().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).unwrap_or_else(|| "N/A".into()));
    println!("  Power State:   {gpu_color}{}{C_RESET}", telemetry.gpu_pstate);
    println!("  Temperature:   {gpu_temp_color}{}°C{C_RESET}", telemetry.gpu_temp);
    println!("  Fan Speed:     {C_BOLD}{}%{C_RESET}", telemetry.gpu_fan_speed);
    println!();

    // Battery Box
    println!("{C_BOLD}► Battery & Power:{C_RESET}");
    println!("  Charge:        {C_BOLD}{}%{C_RESET} ({})", telemetry.battery_capacity, telemetry.battery_status);
    println!("  Health Cap:    {C_CYAN}{}% limit{C_RESET} (Hardware threshold)", telemetry.battery_threshold);
    println!();
}

fn set_mode_cmd(hw: &HardwareController, mode_str: &str) {
    if let Some(mode) = ProfileMode::from_str_loose(mode_str) {
        if let Err(e) = hw.set_mode(mode.clone()) {
            eprintln!("{C_RED}Error applying profile: {e}{C_RESET}");
            return;
        }

        match mode {
            ProfileMode::Silent => {
                println!("{C_GREEN}✓ Switched to SILENT mode (Eco profile, quiet fan curves, Turbo Boost disabled, EPP power).{C_RESET}");
                notify_osd("MSI Profile: Silent", "Eco profile + Quiet fans + Max battery life", "weather-clear-night");
            }
            ProfileMode::Balanced => {
                println!("{C_BLUE}✓ Switched to BALANCED mode (Comfort profile, auto fan control, dynamic clocks).{C_RESET}");
                notify_osd("MSI Profile: Balanced", "Comfort profile + Dynamic fan control", "preferences-system");
            }
            ProfileMode::Turbo => {
                println!("{C_MAGENTA}✓ Switched to TURBO mode (Turbo profile, maximum sustained clocks, EPP performance).{C_RESET}");
                notify_osd("MSI Profile: Turbo", "Maximum performance profile activated", "system-run");
            }
        }
    } else {
        eprintln!("{C_RED}Error: Unknown mode '{mode_str}'. Choose: silent | balanced | turbo{C_RESET}");
    }
}

fn cycle_mode_cmd(hw: &HardwareController) {
    match hw.cycle_mode() {
        Ok(mode) => match mode {
            ProfileMode::Silent => {
                println!("{C_GREEN}✓ Switched to SILENT mode{C_RESET}");
                notify_osd("MSI Profile: Silent", "Eco profile + Quiet fans", "weather-clear-night");
            }
            ProfileMode::Balanced => {
                println!("{C_BLUE}✓ Switched to BALANCED mode{C_RESET}");
                notify_osd("MSI Profile: Balanced", "Comfort profile + Dynamic fans", "preferences-system");
            }
            ProfileMode::Turbo => {
                println!("{C_MAGENTA}✓ Switched to TURBO mode{C_RESET}");
                notify_osd("MSI Profile: Turbo", "Maximum performance profile", "system-run");
            }
        },
        Err(e) => eprintln!("{C_RED}Error cycling mode: {e}{C_RESET}"),
    }
}

fn set_boost_cmd(hw: &HardwareController, action: Option<String>) {
    let result = match action.as_deref() {
        Some("on") | Some("enable") => hw.set_cooler_boost(true).map(|_| true),
        Some("off") | Some("disable") => hw.set_cooler_boost(false).map(|_| false),
        _ => hw.toggle_cooler_boost(),
    };

    match result {
        Ok(true) => {
            println!("{C_RED}❄️ Cooler Boost ENABLED (fans forced to 100% RPM).{C_RESET}");
            notify_osd("Cooler Boost: ON", "Dual fans forced to 100% maximum RPM", "weather-snow");
        }
        Ok(false) => {
            println!("{C_GREEN}❄️ Cooler Boost DISABLED (restored to profile curve).{C_RESET}");
            notify_osd("Cooler Boost: OFF", "Fans restored to profile curve", "weather-clear");
        }
        Err(e) => eprintln!("{C_RED}Error: {e}{C_RESET}"),
    }
}

fn set_battery_cmd(hw: &HardwareController, thresh: Option<u8>) {
    if let Some(t) = thresh {
        if let Err(e) = hw.set_battery_threshold(t) {
            eprintln!("{C_RED}Error setting threshold: {e}{C_RESET}");
        } else {
            println!("{C_GREEN}✓ Battery charge limit set to {t}%.{C_RESET}");
            notify_osd("Battery Threshold", &format!("Charge limit set to {t}%"), "battery");
        }
    } else if let Some(current) = hw.get_battery_threshold() {
        println!("{C_BOLD}Current Battery Charge Limit:{C_RESET} {C_CYAN}{current}%{C_RESET}");
    } else {
        eprintln!("{C_RED}Battery charge limit is not supported or battery not found.{C_RESET}");
    }
}

fn set_cam_cmd(hw: &HardwareController, action: Option<String>) {
    let result = match action.as_deref() {
        Some("block") | Some("off") => hw.set_webcam_block(true).map(|_| true),
        Some("unblock") | Some("on") => hw.set_webcam_block(false).map(|_| false),
        _ => hw.toggle_webcam(),
    };

    match result {
        Ok(true) => {
            println!("{C_RED}📷 Webcam BLOCKED in hardware via EC.{C_RESET}");
            notify_osd("Webcam: Blocked", "Camera hardware-isolated by EC", "camera-disabled");
        }
        Ok(false) => {
            println!("{C_GREEN}📷 Webcam ENABLED.{C_RESET}");
            notify_osd("Webcam: Active", "Camera hardware enabled", "camera-photo");
        }
        Err(e) => eprintln!("{C_RED}Error: {e}{C_RESET}"),
    }
}

fn run_watch(hw: &HardwareController) {
    print!("\x1b[2J"); // Clear screen
    loop {
        print!("\x1b[H"); // Cursor to home
        let telemetry = hw.collect_telemetry();
        print_status(&telemetry, hw);
        println!("{C_DIM}Refreshing every 1s. Press Ctrl+C to exit.{C_RESET}");
        sleep(Duration::from_secs(1));
    }
}

fn main() {
    let cli = Cli::parse();
    let hw = HardwareController::new();

    match cli.command {
        None | Some(Commands::Status) => {
            let telemetry = hw.collect_telemetry();
            print_status(&telemetry, &hw);
        }
        Some(Commands::Mode { mode }) => set_mode_cmd(&hw, &mode),
        Some(Commands::Cycle) => cycle_mode_cmd(&hw),
        Some(Commands::Boost { action }) => set_boost_cmd(&hw, action),
        Some(Commands::Gpu) => {
            let telemetry = hw.collect_telemetry();
            let color = if telemetry.gpu_pstate == "P8" { C_GREEN } else { C_YELLOW };
            println!("{C_BOLD}► NVIDIA GPU Architecture & Power Status:{C_RESET}");
            println!("  PCI Address:   {}", hw.get_gpu_pci_path().map(|p| p.display().to_string()).unwrap_or_else(|| "Not found".into()));
            println!("  Power State:   {color}{}{C_RESET}", telemetry.gpu_pstate);
            println!("  Temperature:   {}°C", telemetry.gpu_temp);
            println!("  Fan Speed:     {}%", telemetry.gpu_fan_speed);
        }
        Some(Commands::Battery { threshold }) => set_battery_cmd(&hw, threshold),
        Some(Commands::Cam { action }) => set_cam_cmd(&hw, action),
        Some(Commands::Watch) => run_watch(&hw),
        Some(Commands::Gui) => {
            #[cfg(feature = "gui")]
            {
                let _ = Command::new("msictl-gui").spawn();
            }
            #[cfg(not(feature = "gui"))]
            {
                eprintln!("{C_RED}Error: msictl was built without GUI support.{C_RESET}");
            }
        }
    }
}
