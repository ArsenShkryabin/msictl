use ksni::blocking::TrayMethods;
use ksni::menu::*;
use ksni::Tray;
use msictl::{HardwareController, ProfileMode};
use slint::ComponentHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

slint::include_modules!();

struct MsiTrayItem {
    hw: Arc<HardwareController>,
    ui_weak: slint::Weak<MainWindow>,
    is_window_visible: Arc<AtomicBool>,
}

impl Tray for MsiTrayItem {
    fn id(&self) -> String {
        "msictl".into()
    }

    fn title(&self) -> String {
        "MSI Control".into()
    }

    fn icon_name(&self) -> String {
        let telem = self.hw.collect_telemetry();
        if telem.cooler_boost {
            "weather-snow".into()
        } else if telem.fan_mode == "silent" || telem.shift_mode == "eco" {
            "weather-clear-night".into()
        } else if telem.shift_mode == "turbo" {
            "system-run".into()
        } else {
            "preferences-system".into()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let visible = self.is_window_visible.load(Ordering::SeqCst);
        let target = !visible;
        self.is_window_visible.store(target, Ordering::SeqCst);

        let ui_weak = self.ui_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak.upgrade() {
                if target {
                    let _ = ui.show();
                } else {
                    let _ = ui.hide();
                }
            }
        });
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let telem = self.hw.collect_telemetry();
        let selected_index = if telem.fan_mode == "silent" || telem.shift_mode == "eco" {
            0
        } else if telem.shift_mode == "turbo" {
            2
        } else {
            1
        };

        let current_mode = match selected_index {
            0 => "Silent",
            2 => "Turbo",
            _ => "Balanced",
        };

        vec![
            StandardItem {
                label: format!("MSI Control ({current_mode})"),
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Show Control Center".into(),
                activate: Box::new(|this: &mut Self| {
                    this.is_window_visible.store(true, Ordering::SeqCst);
                    let ui_weak = this.ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            let _ = ui.show();
                        }
                    });
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            RadioGroup {
                selected: selected_index,
                select: Box::new(|this: &mut Self, idx: usize| {
                    let mode = match idx {
                        0 => ProfileMode::Silent,
                        2 => ProfileMode::Turbo,
                        _ => ProfileMode::Balanced,
                    };
                    let _ = this.hw.set_mode(mode);
                }),
                options: vec![
                    RadioItem {
                        label: "Mode: Silent (Eco & Quiet)".into(),
                        enabled: true,
                        visible: true,
                        ..Default::default()
                    },
                    RadioItem {
                        label: "Mode: Balanced (Dynamic Auto)".into(),
                        enabled: true,
                        visible: true,
                        ..Default::default()
                    },
                    RadioItem {
                        label: "Mode: Turbo (Max Performance)".into(),
                        enabled: true,
                        visible: true,
                        ..Default::default()
                    },
                ],
            }
            .into(),
            MenuItem::Separator,
            CheckmarkItem {
                label: "Cooler Boost (100% Fans)".into(),
                checked: telem.cooler_boost,
                enabled: true,
                visible: true,
                activate: Box::new(|this: &mut Self| {
                    let _ = this.hw.toggle_cooler_boost();
                }),
                ..Default::default()
            }
            .into(),
            CheckmarkItem {
                label: "Block Webcam (Killswitch)".into(),
                checked: telem.webcam_blocked,
                enabled: true,
                visible: true,
                activate: Box::new(|this: &mut Self| {
                    let _ = this.hw.toggle_webcam();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_| {
                    std::process::exit(0);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn sync_telemetry_to_ui(ui: &MainWindow, hw: &HardwareController) {
    let t = hw.collect_telemetry();
    ui.set_cpu_temp(format!("{}°C", t.cpu_temp).into());
    ui.set_cpu_fan(format!("{}%", t.cpu_fan_speed).into());
    ui.set_gpu_temp(format!("{}°C", t.gpu_temp).into());
    ui.set_gpu_fan(format!("{}%", t.gpu_fan_speed).into());
    ui.set_gpu_state(t.gpu_pstate.into());

    let mode_str = if t.fan_mode == "silent" || t.shift_mode == "eco" {
        "silent"
    } else if t.shift_mode == "turbo" || t.fan_mode == "advanced" {
        "turbo"
    } else {
        "balanced"
    };

    ui.set_active_mode(mode_str.into());
    ui.set_cooler_boost(t.cooler_boost);
    ui.set_webcam_blocked(t.webcam_blocked);
    ui.set_battery_status(format!("{}% ({})", t.battery_capacity, t.battery_status).into());
    ui.set_battery_threshold(t.battery_threshold);
}

fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    let hw = Arc::new(HardwareController::new());
    let is_window_visible = Arc::new(AtomicBool::new(true));

    // Ensure Silent (Quiet & Smooth) profile is applied on start
    let _ = hw.set_mode(ProfileMode::Silent);

    // Initial Telemetry Sync
    sync_telemetry_to_ui(&main_window, &hw);

    // Callbacks from UI
    {
        let hw_clone = hw.clone();
        let ui_weak = main_window.as_weak();
        main_window.on_mode_selected(move |mode_str| {
            if let Some(m) = ProfileMode::from_str_loose(&mode_str) {
                let _ = hw_clone.set_mode(m);
                if let Some(ui) = ui_weak.upgrade() {
                    sync_telemetry_to_ui(&ui, &hw_clone);
                }
            }
        });
    }

    {
        let hw_clone = hw.clone();
        let ui_weak = main_window.as_weak();
        main_window.on_cooler_boost_toggled(move || {
            let _ = hw_clone.toggle_cooler_boost();
            if let Some(ui) = ui_weak.upgrade() {
                sync_telemetry_to_ui(&ui, &hw_clone);
            }
        });
    }

    {
        let hw_clone = hw.clone();
        let ui_weak = main_window.as_weak();
        main_window.on_webcam_toggled(move || {
            let _ = hw_clone.toggle_webcam();
            if let Some(ui) = ui_weak.upgrade() {
                sync_telemetry_to_ui(&ui, &hw_clone);
            }
        });
    }

    {
        let hw_clone = hw.clone();
        let ui_weak = main_window.as_weak();
        main_window.on_battery_threshold_changed(move |val| {
            if (50..=100).contains(&val) {
                let _ = hw_clone.set_battery_threshold(val as u8);
                if let Some(ui) = ui_weak.upgrade() {
                    sync_telemetry_to_ui(&ui, &hw_clone);
                }
            }
        });
    }

    {
        let is_visible = is_window_visible.clone();
        let ui_weak = main_window.as_weak();
        main_window.on_hide_window(move || {
            is_visible.store(false, Ordering::SeqCst);
            if let Some(ui) = ui_weak.upgrade() {
                let _ = ui.hide();
            }
        });
    }

    // Start background StatusNotifierItem (Tray)
    let tray_item = MsiTrayItem {
        hw: hw.clone(),
        ui_weak: main_window.as_weak(),
        is_window_visible: is_window_visible.clone(),
    };

    let tray_handle = tray_item.spawn().expect("Failed to initialize system tray");

    // Periodic telemetry updater in background
    {
        let hw_clone = hw.clone();
        let ui_weak = main_window.as_weak();
        let is_visible = is_window_visible.clone();

        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(1500));
            tray_handle.update(|_| {});

            if is_visible.load(Ordering::SeqCst) {
                let hw = hw_clone.clone();
                let ui_weak = ui_weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        sync_telemetry_to_ui(&ui, &hw);
                    }
                });
            }
        });
    }

    main_window.run()
}
