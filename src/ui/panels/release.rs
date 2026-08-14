use crate::app::autoupdate::Release;
use crate::app::state::{ReleaseNotesStatus, TabletMapperApp};
use crate::t;
use eframe::egui;

struct ParsedRelease {
    version: String,
    date: String,
    additions: Vec<String>,
    removals: Vec<String>,
    fixes: Vec<String>,
    improvements: Vec<String>,
    notes: Vec<String>,
    videos: Vec<String>,
    contributors: Vec<String>,
}

fn format_date(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map_or_else(|_| iso.to_string(), |dt| dt.format("%d/%m/%Y").to_string())
}

fn find_youtube_url(line: &str) -> Option<String> {
    line.split_whitespace().find_map(|word| {
        let trimmed = word.trim_matches(|c: char| matches!(c, '(' | ')' | '[' | ']' | ',' | '<' | '>'));
        let is_http = trimmed.starts_with("http://") || trimmed.starts_with("https://");
        let is_youtube = trimmed.contains("youtube.com/watch")
            || trimmed.contains("youtu.be/")
            || trimmed.contains("youtube.com/embed");
        (is_http && is_youtube).then(|| trimmed.to_string())
    })
}

fn mention_username(word: &str) -> Option<&str> {
    let trimmed = word.trim_end_matches([',', '.', ')', ':', ';', '!', '?']);
    let name = trimmed.strip_prefix('@')?;
    let valid = !name.is_empty()
        && name.len() <= 39
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-');
    valid.then_some(name)
}

fn strip_category_prefix<'a>(line: &'a str, lower: &str, prefix: &str) -> Option<&'a str> {
    if lower.starts_with(prefix) {
        line.get(prefix.len()..).map(str::trim)
    } else {
        None
    }
}

fn parse_release(release: &Release) -> ParsedRelease {
    let mut additions = Vec::new();
    let mut removals = Vec::new();
    let mut fixes = Vec::new();
    let mut improvements = Vec::new();
    let mut notes = Vec::new();
    let mut videos = Vec::new();
    let mut contributors = Vec::new();

    let body = release.body.as_deref().unwrap_or_default();
    for raw_line in body.lines() {
        let trimmed = raw_line.trim().trim_start_matches(['-', '*', '•']).trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let cleaned = trimmed.replace("**", "");
        let line = cleaned.trim();

        for word in line.split_whitespace() {
            if let Some(user) = mention_username(word)
                && !contributors
                    .iter()
                    .any(|c: &String| c.eq_ignore_ascii_case(user))
            {
                contributors.push(user.to_string());
            }
        }

        if let Some(url) = find_youtube_url(line) {
            videos.push(url);
            continue;
        }

        let lower = line.to_lowercase();
        if let Some(rest) = strip_category_prefix(line, &lower, "add:") {
            additions.push(rest.to_string());
        } else if let Some(rest) = strip_category_prefix(line, &lower, "fix:") {
            fixes.push(rest.to_string());
        } else if let Some(rest) = strip_category_prefix(line, &lower, "improve:")
            .or_else(|| strip_category_prefix(line, &lower, "improvement:"))
        {
            improvements.push(rest.to_string());
        } else if let Some(rest) = strip_category_prefix(line, &lower, "remove:")
            .or_else(|| strip_category_prefix(line, &lower, "delete:"))
        {
            removals.push(rest.to_string());
        } else if let Some(rest) = strip_category_prefix(line, &lower, "info:") {
            notes.push(rest.to_string());
        } else {
            notes.push(line.to_string());
        }
    }

    ParsedRelease {
        version: release.tag_name.trim_start_matches('v').to_string(),
        date: release.published_at.as_deref().map_or_else(String::new, format_date),
        additions,
        removals,
        fixes,
        improvements,
        notes,
        videos,
        contributors,
    }
}

pub fn render_release_panel(app: &TabletMapperApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.label(egui::RichText::new(t!("release.title")).size(24.0).strong());
        ui.add_space(15.0);
    });

    match &app.release_notes.status {
        ReleaseNotesStatus::Idle | ReleaseNotesStatus::Loading => {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.spinner();
                ui.add_space(8.0);
                ui.label(t!("release.loading"));
            });
        }
        ReleaseNotesStatus::Unavailable => {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(
                    egui::RichText::new(t!("release.unavailable"))
                        .color(crate::ui::theme::semantic_colors(ui.ctx()).error),
                );
            });
        }
        ReleaseNotesStatus::Loaded { releases, from_cache } => {
            if *from_cache {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(t!("release.cached_notice"))
                            .weak()
                            .italics(),
                    );
                });
                ui.add_space(8.0);
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.add_space(10.0);
                    for release in releases {
                        let parsed = parse_release(release);
                        render_release_entry(ui, &parsed);
                        ui.add_space(20.0);
                    }
                    ui.add_space(10.0);
                });
        }
    }
}

fn render_release_entry(ui: &mut egui::Ui, entry: &ParsedRelease) {
    let visuals = ui.visuals();
    let card_bg = visuals.window_fill.gamma_multiply(0.6);
    let border_color = visuals
        .widgets
        .noninteractive
        .bg_stroke
        .color
        .gamma_multiply(0.4);
    let text_color = visuals.text_color();

    egui::Frame::new()
        .fill(card_bg)
        .corner_radius(4.0)
        .stroke(egui::Stroke::new(1.0_f32, border_color))
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
                        ui.label(egui::RichText::new(&entry.date).weak().size(12.0));
                    });
                });

                ui.add_space(12.0);

                let semantic = crate::ui::theme::semantic_colors(ui.ctx());
                render_category(
                    ui,
                    "NEW",
                    egui_phosphor::regular::PLUS_CIRCLE,
                    semantic.success,
                    &entry.additions,
                );
                render_category(
                    ui,
                    "FIX",
                    egui_phosphor::regular::WRENCH,
                    semantic.warning,
                    &entry.fixes,
                );
                render_category(
                    ui,
                    "IMP",
                    egui_phosphor::regular::CHART_LINE_UP,
                    semantic.info,
                    &entry.improvements,
                );
                render_category(
                    ui,
                    "DEL",
                    egui_phosphor::regular::MINUS_CIRCLE,
                    semantic.error,
                    &entry.removals,
                );
                render_category(
                    ui,
                    "INFO",
                    egui_phosphor::regular::INFO,
                    text_color,
                    &entry.notes,
                );

                render_videos(ui, &entry.videos);
                render_contributors(ui, &entry.contributors);
            });
        });
}

fn render_category(ui: &mut egui::Ui, label: &str, icon: &str, color: egui::Color32, items: &[String]) {
    if items.is_empty() {
        return;
    }

    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(color.gamma_multiply(0.1))
            .stroke(egui::Stroke::new(1.0_f32, color.gamma_multiply(0.5)))
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

    let item_color = ui.visuals().text_color().gamma_multiply(0.8);
    for item in items {
        render_bullet_text(ui, item, item_color);
        ui.add_space(2.0);
    }

    ui.add_space(10.0);
}

fn render_bullet_text(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add_space(8.0);
        ui.label(egui::RichText::new(egui_phosphor::regular::CARET_RIGHT).color(color));
        for word in text.split_whitespace() {
            if let Some(username) = mention_username(word) {
                ui.hyperlink_to(
                    egui::RichText::new(word).size(12.5),
                    format!("https://github.com/{username}"),
                );
            } else {
                ui.label(egui::RichText::new(word).size(12.5).color(color));
            }
        }
    });
}

fn render_contributors(ui: &mut egui::Ui, usernames: &[String]) {
    if usernames.is_empty() {
        return;
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(8.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 6.0);
        ui.label(
            egui::RichText::new(t!("release.contributors"))
                .weak()
                .size(11.0),
        );
        for username in usernames {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add(
                    egui::Image::from_uri(format!("https://github.com/{username}.png?size=64"))
                        .fit_to_exact_size(egui::vec2(20.0, 20.0))
                        .corner_radius(10.0),
                );
                ui.hyperlink_to(
                    egui::RichText::new(format!("@{username}")).size(12.0),
                    format!("https://github.com/{username}"),
                );
            });
        }
    });
}

fn render_videos(ui: &mut egui::Ui, urls: &[String]) {
    if urls.is_empty() {
        return;
    }

    ui.add_space(4.0);
    for url in urls {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            if ui
                .button(format!(
                    "{} {}",
                    egui_phosphor::regular::YOUTUBE_LOGO,
                    t!("release.watch_video")
                ))
                .clicked()
            {
                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            }
        });
        ui.add_space(4.0);
    }
}
