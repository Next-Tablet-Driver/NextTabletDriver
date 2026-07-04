use eframe::egui;

struct ReleaseEntry {
    version: &'static str,
    date: &'static str,
    additions: &'static [&'static str],
    removals: &'static [&'static str],
    fixes: &'static [&'static str],
    improvements: &'static [&'static str],
}

const RELEASES: &[ReleaseEntry] = &[
    ReleaseEntry {
        version: "1.26.0505.01",
        date: "05/05/2026",
        additions: &[
            "Add: Robust auto-updater with versioning, progress tracking, and SHA verification",
            "Add: User presets options for customized configuration",
            "Add: Dedicated 'Help' section and expanded logging groups",
            "Add: Cross-compilation support via Cross.toml integration",
        ],
        removals: &[],
        fixes: &[
            "Fix: Immediate 'Out of Range' detection for XP-PEN tablets",
            "Fix: Panic safety by replacing all '.unwrap()' calls with robust error handling",
            "Fix: Evdev syntax and auto-updater logic regression",
            "Fix: Sidebar layout margins and UI theme inconsistencies",
            "Fix: Pipeline execution errors and failing unit tests",
        ],
        improvements: &[
            "Improve: Decoupled UI from SharedState using the Snapshot pattern for better state management",
            "Improve: Strict separation of domain logic from the presentation layer",
            "Improve: Migration to Rust 1.96.1 toolchain for improved stability",
            "Improve: Pipeline performance optimizations and Filters UI refinements",
            "Improve: General codebase health through formatting and syntax standardisation",
        ],
    },
    ReleaseEntry {
        version: "1.26.2004.01",
        date: "20/04/2026",
        additions: &[
            "Add: 'Developer' debug tab for real-time pipeline telemetry and HID packet inspection",
            "Add: Extensive support for new tablet drivers (Acepen, Bosto, Floogoo, Genius, Lifetec, Robotpen, Tenmoon, UC-Logic, ViewSonic, and numerous Wacom models)",
        ],
        removals: &["Remove: Support panel"],
        fixes: &[
            "Fix: Cursor teleportation bug occurring upon tablet connection",
            "Fix: Input not registering at the exact [0, 0] coordinate",
            "Fix: Pen hover distance detection (out-of-range) not properly clearing state on timeout",
        ],
        improvements: &[
            "Improve: Complete refactoring and modularization of tablet driver parsers",
            "Improve: Debugger UI responsiveness with optimized 16ms temporal throttling",
            "Improve: Codebase maintainability through comprehensive comment auditing and cleanup",
        ],
    },
    ReleaseEntry {
        version: "1.26.2103.02",
        date: "21/03/2026",
        additions: &[],
        removals: &[],
        fixes: &[
            "Updated to Rust 2024 edition",
            "Updated all dependencies to their latest versions",
            "Fixed a security vulnerability detected in dependencies",
            "Adapted codebase to new APIs introduced by dependency updates",
        ],
        improvements: &[],
    },
    ReleaseEntry {
        version: "1.26.2103.01",
        date: "21/03/2026",
        additions: &[
            "Add: Added 4 Catppuccin themes (Latte, Frappe, Macchiato, Mocha)",
            "Add: Added Osu! Playfield preview in the mapping area",
        ],
        removals: &[],
        fixes: &[
            "Fix: Cleaned up and modernized the default egui UI design (borders, rounding, and hover effects)",
        ],
        improvements: &["Improve: Improved theme-awareness for custom UI components"],
    },
    ReleaseEntry {
        version: "1.26.2003.01",
        date: "20/03/2026",
        additions: &["Add: 'Theme' settings in 'Settings' tab"],
        removals: &[],
        fixes: &[],
        improvements: &["Improve: 'Theme' settings to allow changing the theme of the application"],
    },
    ReleaseEntry {
        version: "1.26.1903.03",
        date: "19/03/2026",
        additions: &[],
        removals: &["Remove: Powershell files, Payload.json"],
        fixes: &[],
        improvements: &[
            "Next Tablet Driver now has a GitHub organization, the project has also been cleaned up for a better presentation in the future.",
        ],
    },
    ReleaseEntry {
        version: "1.26.1303.03",
        date: "13/03/2026",
        additions: &[],
        removals: &["Remove: Telemetry System"],
        fixes: &[],
        improvements: &["Internal Documentation"],
    },
    ReleaseEntry {
        version: "1.26.1203.01",
        date: "12/03/2026",
        additions: &[
            "Add: 'Relative Mode' for pen input",
            "Add: 'Filters' tab and 'Devocub Antichatter' settings like Open Tablet Driver Filters and 'HandSpeed WebSocket' settings",
        ],
        removals: &["Remove: Crypto Donations", "Remove: 'Tools' tab"],
        fixes: &["Nothing"],
        improvements: &[
            "Improve: 'HandSpeed WebSocket' filter to send 'total_distance' in addition to 'handspeed'",
        ],
    },
    ReleaseEntry {
        version: "1.26.0503.01",
        date: "05/03/2026",
        additions: &[
            "Add: Telemetry System for improvement (you can disable it in 'Settings' tab)",
            "Add: 'Relative Mode' for pen input",
            "Info: The telemetry doesn't collect any personally identifiable information; it’s only there to improve the driver. An example of the shared data is available to view on GitHub.",
        ],
        removals: &["Nothing"],
        fixes: &["Change version format to European format instead of US format (MMDD -> DDMM)"],
        improvements: &["Nothing"],
    },
    ReleaseEntry {
        version: "1.26.0303.03",
        date: "03/03/2026",
        additions: &[
            "New 'Release' tab to track changes",
            "Added Support & Contribution panel with Crypto donations",
        ],
        removals: &["Nothing"],
        fixes: &["Fix all 'cargo clippy' issues and warnings (as mentioned in ISSUE#2)"],
        improvements: &[
            "Add 'CI/CD' pipeline for automated code quality checks (as mentioned in ISSUE#1)",
        ],
    },
    ReleaseEntry {
        version: "1.26.0301.05",
        date: "01/03/2026",
        additions: &["New 'Websocket Server' settings in 'Settings' tab"],
        removals: &["Nothing"],
        fixes: &[
            "Improved 'Run At Startup' feature, before it was not working properly and flagged by Windows Defender",
            "Improved HID API initialization performance",
        ],
        improvements: &["Event-driven architecture for reduced CPU usage"],
    },
];

pub fn render_release_panel(_app: &crate::app::state::TabletMapperApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.label(egui::RichText::new("Release History").size(24.0).strong());
        ui.add_space(15.0);
    });

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.add_space(10.0);
            for release in RELEASES {
                render_release_entry(ui, release);
                ui.add_space(20.0);
            }
            ui.add_space(10.0);
        });
}

fn render_release_entry(ui: &mut egui::Ui, entry: &ReleaseEntry) {
    let visuals = ui.visuals();
    let card_bg = visuals.window_fill.gamma_multiply(0.6);
    let border_color = visuals
        .widgets
        .noninteractive
        .bg_stroke
        .color
        .gamma_multiply(0.4);

    egui::Frame::new()
        .fill(card_bg)
        .corner_radius(4.0)
        .stroke(egui::Stroke::new(1.0, border_color))
        .inner_margin(egui::Margin::symmetric(20, 15))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Next Tablet Driver | v{}", entry.version))
                            .size(16.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(entry.date).weak().size(12.0));
                    });
                });

                ui.add_space(12.0);

                let semantic = crate::ui::theme::semantic_colors(ui.ctx());
                render_category(
                    ui,
                    "NEW",
                    egui_phosphor::regular::PLUS_CIRCLE,
                    semantic.success,
                    entry.additions,
                );
                render_category(
                    ui,
                    "FIX",
                    egui_phosphor::regular::WRENCH,
                    semantic.warning,
                    entry.fixes,
                );
                render_category(
                    ui,
                    "IMP",
                    egui_phosphor::regular::CHART_LINE_UP,
                    semantic.info,
                    entry.improvements,
                );
                render_category(
                    ui,
                    "DEL",
                    egui_phosphor::regular::MINUS_CIRCLE,
                    semantic.error,
                    entry.removals,
                );
            });
        });
}

fn render_category(
    ui: &mut egui::Ui,
    label: &str,
    icon: &str,
    color: egui::Color32,
    items: &[&str],
) {
    if items.is_empty() {
        return;
    }

    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(color.gamma_multiply(0.1))
            .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.5)))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!("{icon} {label}"))
                        .color(color)
                        .size(10.0)
                        .strong(),
                );
            });
    });

    ui.add_space(4.0);

    for item in items {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(egui_phosphor::regular::CARET_RIGHT);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(*item)
                    .size(12.5)
                    .color(ui.visuals().text_color().gamma_multiply(0.8)),
            );
        });
        ui.add_space(2.0);
    }

    ui.add_space(10.0);
}
