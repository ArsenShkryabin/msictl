use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const MSI_EC_PATH: &str = "/sys/devices/platform/msi-ec";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileMode {
    Silent,
    Balanced,
    Turbo,
}

impl ProfileMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Silent => "silent",
            Self::Balanced => "balanced",
            Self::Turbo => "turbo",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "silent" | "quiet" | "eco" => Some(Self::Silent),
            "balanced" | "comfort" | "auto" => Some(Self::Balanced),
            "turbo" | "performance" | "max" => Some(Self::Turbo),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SystemTelemetry {
    pub fw_version: String,
    pub fan_mode: String,
    pub shift_mode: String,
    pub cooler_boost: bool,
    pub webcam_blocked: bool,
    pub cpu_temp: i32,
    pub cpu_fan_speed: i32,
    pub gpu_temp: i32,
    pub gpu_fan_speed: i32,
    pub battery_capacity: i32,
    pub battery_status: String,
    pub battery_threshold: i32,
    pub gpu_pstate: String,
    pub gpu_name: String,
    pub gpu_active: bool,
}

pub struct HardwareController {
    battery_path: Option<PathBuf>,
    gpu_pci_path: Option<PathBuf>,
}

impl HardwareController {
    pub fn new() -> Self {
        let battery_path = Self::find_battery_path();
        let gpu_pci_path = Self::find_nvidia_gpu_path();

        Self {
            battery_path,
            gpu_pci_path,
        }
    }

    /// Dynamically locates the primary ACPI battery (BAT0, BAT1, etc.)
    pub fn find_battery_path() -> Option<PathBuf> {
        let ps_dir = Path::new("/sys/class/power_supply");
        if let Ok(entries) = fs::read_dir(ps_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("BAT") {
                    let path = entry.path();
                    let type_file = path.join("type");
                    if let Ok(ty) = fs::read_to_string(type_file) {
                        if ty.trim().eq_ignore_ascii_case("battery") {
                            return Some(path);
                        }
                    }
                    return Some(path);
                }
            }
        }
        None
    }

    /// Dynamically discovers NVIDIA discrete GPU by scanning PCI vendor ID 0x10de and VGA/3D class
    pub fn find_nvidia_gpu_path() -> Option<PathBuf> {
        let pci_dir = Path::new("/sys/bus/pci/devices");
        if let Ok(entries) = fs::read_dir(pci_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let vendor_file = path.join("vendor");
                let class_file = path.join("class");

                if let (Ok(vendor), Ok(class)) = (fs::read_to_string(vendor_file), fs::read_to_string(class_file)) {
                    let vendor = vendor.trim().to_lowercase();
                    let class = class.trim().to_lowercase();

                    // 0x10de is NVIDIA
                    if vendor == "0x10de" {
                        // Class 0x03xxxx represents Display controller (0x0300 VGA, 0x0302 3D Controller)
                        if class.starts_with("0x03") {
                            return Some(path);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn read_sysfs(path: &Path) -> Option<String> {
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    pub fn write_sysfs(path: &Path, value: &str) -> Result<(), String> {
        if !path.exists() {
            return Err(format!("File does not exist: {}", path.display()));
        }

        match fs::write(path, value) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Fallback via sudo if permissions are denied
                let status = Command::new("sudo")
                    .args(["sh", "-c", &format!("echo '{}' > '{}'", value, path.display())])
                    .status()
                    .map_err(|se| format!("Failed to run sudo: {se}"))?;

                if status.success() {
                    Ok(())
                } else {
                    Err(format!("Permission denied writing to {} ({e})", path.display()))
                }
            }
        }
    }

    pub fn is_msi_ec_available() -> bool {
        Path::new(MSI_EC_PATH).exists()
    }

    pub fn collect_telemetry(&self) -> SystemTelemetry {
        let ec = Path::new(MSI_EC_PATH);
        let fw_version = Self::read_sysfs(&ec.join("fw_version")).unwrap_or_else(|| "N/A".into());
        let fan_mode = Self::read_sysfs(&ec.join("fan_mode")).unwrap_or_else(|| "auto".into());
        let shift_mode = Self::read_sysfs(&ec.join("shift_mode")).unwrap_or_else(|| "auto".into());
        let cooler_boost = Self::read_sysfs(&ec.join("cooler_boost")).map(|s| s == "on").unwrap_or(false);
        let webcam_blocked = Self::read_sysfs(&ec.join("webcam_block")).map(|s| s == "on").unwrap_or(false);

        let cpu_temp = Self::read_sysfs(&ec.join("cpu/realtime_temperature"))
            .and_then(|s| s.parse().ok()).unwrap_or(0);
        let cpu_fan_speed = Self::read_sysfs(&ec.join("cpu/realtime_fan_speed"))
            .and_then(|s| s.parse().ok()).unwrap_or(0);

        let gpu_temp = Self::read_sysfs(&ec.join("gpu/realtime_temperature"))
            .and_then(|s| s.parse().ok()).unwrap_or(0);
        let gpu_fan_speed = Self::read_sysfs(&ec.join("gpu/realtime_fan_speed"))
            .and_then(|s| s.parse().ok()).unwrap_or(0);

        let mut battery_capacity = 0;
        let mut battery_status = "Unknown".to_string();
        let mut battery_threshold = 100;

        if let Some(ref bat) = self.battery_path {
            battery_capacity = Self::read_sysfs(&bat.join("capacity"))
                .and_then(|s| s.parse().ok()).unwrap_or(0);
            battery_status = Self::read_sysfs(&bat.join("status")).unwrap_or_else(|| "Unknown".into());
            battery_threshold = Self::read_sysfs(&bat.join("charge_control_end_threshold"))
                .and_then(|s| s.parse().ok()).unwrap_or(100);
        }

        let mut gpu_pstate = "Offline".to_string();
        let mut gpu_active = false;
        let gpu_name = "NVIDIA dGPU".to_string();

        if self.gpu_pci_path.as_ref().map(|p| p.exists()).unwrap_or(false) {
            gpu_active = true;
            if let Ok(output) = Command::new("nvidia-smi")
                .args(["--query-gpu=pstate", "--format=csv,noheader,nounits"])
                .output()
            {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    let st = s.trim().to_string();
                    if !st.is_empty() {
                        gpu_pstate = st;
                    }
                }
            } else {
                gpu_pstate = "Active".into();
            }
        }

        SystemTelemetry {
            fw_version,
            fan_mode,
            shift_mode,
            cooler_boost,
            webcam_blocked,
            cpu_temp,
            cpu_fan_speed,
            gpu_temp,
            gpu_fan_speed,
            battery_capacity,
            battery_status,
            battery_threshold,
            gpu_pstate,
            gpu_name,
            gpu_active,
        }
    }

    pub fn set_cpu_tuning(no_turbo: bool, epp: &str) {
        let _ = Self::write_sysfs(
            Path::new("/sys/devices/system/cpu/intel_pstate/no_turbo"),
            if no_turbo { "1" } else { "0" },
        );

        if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu") {
            for entry in entries.flatten() {
                let path = entry.path().join("cpufreq/energy_performance_preference");
                if path.exists() {
                    let _ = Self::write_sysfs(&path, epp);
                }
            }
        }
    }

    pub fn set_mode(&self, mode: ProfileMode) -> Result<(), String> {
        let ec = Path::new(MSI_EC_PATH);
        match mode {
            ProfileMode::Silent => {
                Self::write_sysfs(&ec.join("cooler_boost"), "off")?;
                Self::write_sysfs(&ec.join("fan_mode"), "silent")?;
                Self::write_sysfs(&ec.join("shift_mode"), "eco")?;
                // Use balance_power instead of power to avoid 800MHz throttling and GUI stutters
                Self::set_cpu_tuning(true, "balance_power");
            }
            ProfileMode::Balanced => {
                Self::write_sysfs(&ec.join("cooler_boost"), "off")?;
                Self::write_sysfs(&ec.join("fan_mode"), "auto")?;
                Self::write_sysfs(&ec.join("shift_mode"), "comfort")?;
                Self::set_cpu_tuning(false, "balance_power");
            }
            ProfileMode::Turbo => {
                Self::write_sysfs(&ec.join("cooler_boost"), "off")?;
                Self::write_sysfs(&ec.join("fan_mode"), "advanced")?;
                Self::write_sysfs(&ec.join("shift_mode"), "turbo")?;
                Self::set_cpu_tuning(false, "performance");
            }
        }
        Ok(())
    }

    pub fn cycle_mode(&self) -> Result<ProfileMode, String> {
        let ec = Path::new(MSI_EC_PATH);
        let fan_mode = Self::read_sysfs(&ec.join("fan_mode")).unwrap_or_default();
        let shift_mode = Self::read_sysfs(&ec.join("shift_mode")).unwrap_or_default();

        let next = if fan_mode == "silent" || shift_mode == "eco" {
            ProfileMode::Balanced
        } else if shift_mode == "comfort" || fan_mode == "auto" {
            ProfileMode::Turbo
        } else {
            ProfileMode::Silent
        };

        self.set_mode(next.clone())?;
        Ok(next)
    }

    pub fn set_cooler_boost(&self, enable: bool) -> Result<(), String> {
        let ec = Path::new(MSI_EC_PATH);
        Self::write_sysfs(&ec.join("cooler_boost"), if enable { "on" } else { "off" })
    }

    pub fn toggle_cooler_boost(&self) -> Result<bool, String> {
        let ec = Path::new(MSI_EC_PATH);
        let current = Self::read_sysfs(&ec.join("cooler_boost")).unwrap_or_else(|| "off".into());
        let target = current != "on";
        self.set_cooler_boost(target)?;
        Ok(target)
    }

    pub fn set_webcam_block(&self, block: bool) -> Result<(), String> {
        let ec = Path::new(MSI_EC_PATH);
        Self::write_sysfs(&ec.join("webcam_block"), if block { "on" } else { "off" })
    }

    pub fn toggle_webcam(&self) -> Result<bool, String> {
        let ec = Path::new(MSI_EC_PATH);
        let current = Self::read_sysfs(&ec.join("webcam_block")).unwrap_or_else(|| "off".into());
        let target = current != "on"; // if currently blocked, unblock (target=false)
        self.set_webcam_block(target)?;
        Ok(target)
    }

    pub fn set_battery_threshold(&self, threshold: u8) -> Result<(), String> {
        if !(50..=100).contains(&threshold) {
            return Err("Threshold must be between 50% and 100%".into());
        }

        if let Some(ref bat) = self.battery_path {
            let path = bat.join("charge_control_end_threshold");
            Self::write_sysfs(&path, &threshold.to_string())
        } else {
            Err("No supported ACPI battery found with charge threshold control".into())
        }
    }

    pub fn get_battery_threshold(&self) -> Option<u8> {
        self.battery_path
            .as_ref()
            .and_then(|bat| Self::read_sysfs(&bat.join("charge_control_end_threshold")))
            .and_then(|s| s.parse().ok())
    }

    pub fn get_gpu_pci_path(&self) -> Option<&Path> {
        self.gpu_pci_path.as_deref()
    }

    pub fn get_battery_path(&self) -> Option<&Path> {
        self.battery_path.as_deref()
    }
}
