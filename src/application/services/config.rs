use crate::core::config::models::MappingConfig;
use crate::settings::load_last_session;

pub struct ConfigService {
    pub config: MappingConfig,
    pub corrections: Vec<String>,
}

impl ConfigService {
    #[must_use]
    pub fn load() -> Self {
        let loaded = load_last_session();
        let (config, corrections) = if let Some((cfg, corrections)) = loaded {
            log::info!(target: "Config", "Using loaded configuration from last session");
            (cfg, corrections)
        } else {
            let cfg = MappingConfig {
                run_at_startup: crate::startup::is_run_at_startup_registered(),
                ..Default::default()
            };
            (cfg, Vec::new())
        };
        Self {
            config,
            corrections,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::settings::{save_last_session, set_test_settings_dir};

    fn temp_settings(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ntd_config_service_{name}_{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        set_test_settings_dir(dir.clone());
        dir
    }

    #[test]
    fn without_a_saved_session_the_defaults_are_used() {
        let dir = temp_settings("fresh");
        let service = ConfigService::load();
        assert!(service.corrections.is_empty());
        assert_eq!(
            service.config.active_area,
            MappingConfig::default().active_area
        );
        assert_eq!(
            service.config.run_at_startup,
            crate::startup::is_run_at_startup_registered()
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_last_session_is_restored() {
        let dir = temp_settings("restore");
        let mut saved = MappingConfig::default();
        saved.active_area.w = 77.0;
        save_last_session(&saved).unwrap();

        let service = ConfigService::load();
        assert_eq!(service.config.active_area.w, 77.0);
        assert!(service.corrections.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_damaged_session_is_repaired_and_the_repairs_are_reported() {
        let dir = temp_settings("repair");
        let mut saved = MappingConfig::default();
        saved.active_area.w = -5.0;
        save_last_session(&saved).unwrap();

        let service = ConfigService::load();
        assert!(service.config.active_area.w > 0.0);
        assert_eq!(service.corrections.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
