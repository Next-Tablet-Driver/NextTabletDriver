//! Diagnostic logging of the active mapping configuration.

use crate::core::config::models::MappingConfig;

/// Logs the current driver mapping configuration settings to the tracking log target.
pub fn log_mapping_config(config: &MappingConfig, prefix: &str) {
    log::info!(target: "Tracking", "=== CONFIGURATION LOG ({prefix}) ===");
    log::info!(target: "Tracking", "Mode: {:?}", config.mode);
    log::info!(
        target: "Tracking",
        "Active Area -> Width: {:.2}, Height: {:.2} | Offsets -> X: {:.2}, Y: {:.2} | Rotation: {:.1} deg",
        config.active_area.w,
        config.active_area.h,
        config.active_area.x,
        config.active_area.y,
        config.active_area.rotation
    );
    log::info!(
        target: "Tracking",
        "Target Area -> Width: {:.2}, Height: {:.2} | Offsets -> X: {:.2}, Y: {:.2}",
        config.target_area.w,
        config.target_area.h,
        config.target_area.x,
        config.target_area.y
    );
    log::info!(
        target: "Tracking",
        "Antichatter -> Enabled: {} | Latency: {:.1}ms | Strength: {:.2}",
        config.antichatter.enabled,
        config.antichatter.latency,
        config.antichatter.antichatter_strength
    );
    log::info!(
        target: "Tracking",
        "Stylus -> Tip Threshold: {} | Eraser Threshold: {} | Disable Pressure: {} | Disable Tilt: {}",
        config.tip_threshold,
        config.eraser_threshold,
        config.disable_pressure,
        config.disable_tilt
    );
    log::info!(
        target: "Tracking",
        "General -> Lock Aspect Ratio: {} | Show Playfield: {}",
        config.lock_aspect_ratio,
        config.show_osu_playfield
    );
}
