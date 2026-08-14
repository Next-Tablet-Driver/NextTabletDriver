pub mod antichatter;
pub mod stats;

use crate::app::state::{TabletMapperApp, UiSnapshot};
use crate::core::config::models::MappingConfig;
use crate::t;
use crate::ui::theme::{panel_bg, panel_border};
use eframe::egui;

/// A single entry in the filter registry: how it appears in the sidebar,
/// the id used to track selection, and how to render its settings.
///
/// Adding a new filter means adding one entry here, nothing else.
struct FilterEntry {
    name: &'static str,
    icon: &'static str,
    id: &'static str,
    render: fn(&TabletMapperApp, &mut egui::Ui, &mut MappingConfig, &UiSnapshot),
}

fn filter_registry() -> [FilterEntry; 2] {
    [
        FilterEntry {
            name: "Antichatter",
            icon: egui_phosphor::regular::WAVE_SINE,
            id: "Devocub Antichatter",
            render: |_app, ui, config, _snapshot| {
                antichatter::render_antichatter_settings(ui, config);
            },
        },
        FilterEntry {
            name: "HandSpeed",
            icon: egui_phosphor::regular::GAUGE,
            id: "HandSpeed WebSocket",
            render: stats::render_stats_settings,
        },
    ]
}

pub fn render_filters_panel(
    app: &mut TabletMapperApp,
    ui: &mut egui::Ui,
    config: &mut MappingConfig,
    snapshot: &UiSnapshot,
) {
    let registry = filter_registry();

    ui.add_space(5.0);
    ui.horizontal(|ui| {
        let sidebar_width = 150.0;
        let sidebar_height = ui.available_height() - 10.0;

        ui.allocate_ui_with_layout(
            egui::vec2(sidebar_width, sidebar_height),
            egui::Layout::top_down_justified(egui::Align::LEFT),
            |ui| {
                let visuals = ui.visuals();
                egui::Frame::new()
                    .fill(panel_bg(visuals).gamma_multiply(0.6))
                    .stroke(egui::Stroke::new(
                        1.0,
                        panel_border(visuals).gamma_multiply(0.4),
                    ))
                    .inner_margin(egui::Margin::symmetric(10, 10))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.set_min_height(sidebar_height);
                        ui.spacing_mut().item_spacing.y = 2.0;

                        ui.label(
                            egui::RichText::new(t!("filters.available"))
                                .weak()
                                .size(10.0),
                        );
                        ui.add_space(8.0);

                        for (i, entry) in registry.iter().enumerate() {
                            if i > 0 {
                                ui.add_space(4.0);
                            }
                            render_sidebar_item(
                                ui,
                                entry.name,
                                entry.icon,
                                entry.id,
                                &mut app.selected_filter,
                            );
                        }
                    });
            },
        );

        ui.add_space(12.0);

        ui.vertical(|ui| {
            ui.add_space(5.0);
            match registry
                .iter()
                .find(|entry| entry.id == app.selected_filter)
            {
                Some(entry) => (entry.render)(app, ui, config, snapshot),
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label(t!("filters.select"));
                    });
                }
            }
        });
    });
}

fn render_sidebar_item(
    ui: &mut egui::Ui,
    name: &str,
    icon: &str,
    filter_id: &str,
    selected: &mut String,
) {
    let is_selected = selected == filter_id;
    let (text_color, bg_color, selection_color) = {
        let visuals = ui.visuals();
        let tc = if is_selected {
            visuals.strong_text_color()
        } else {
            visuals.text_color().gamma_multiply(0.6)
        };
        let bg = if is_selected {
            visuals.widgets.hovered.bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        (tc, bg, visuals.selection.bg_fill)
    };

    let margin = 10.0;
    let item_width = ui.available_width() - margin * 2.0;

    let response = ui
        .scope(|ui| {
            ui.set_width(item_width);
            egui::Frame::new()
                .fill(bg_color)
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.set_width(item_width);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(icon).color(text_color).size(14.0));
                        ui.label(egui::RichText::new(name).color(text_color).strong());
                    });
                })
        })
        .response;

    let response = ui.interact(response.rect, ui.id().with(filter_id), egui::Sense::click());
    if response.clicked() {
        *selected = filter_id.to_string();
    }

    if is_selected {
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(response.rect.left() - 10.0, response.rect.top() + 8.0),
                egui::pos2(response.rect.left() - 7.0, response.rect.bottom() - 8.0),
            ),
            2.0,
            selection_color,
        );
    }
}
