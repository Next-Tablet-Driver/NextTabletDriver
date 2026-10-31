use ntd_core::config::models::MappingConfig;
use ntd_settings::{Paths, load_last_session};

/// Loads the configuration the application starts with.
pub struct ConfigService {
    /// The configuration to start with: the one of the last session, or the defaults.
    pub config: MappingConfig,
    /// Repairs made while loading, to report to the user.
    pub corrections: Vec<String>,
}

impl ConfigService {
    /// Loads the configuration of the last session, or the defaults when there is none.
    #[must_use]
    pub fn load(paths: &Paths) -> Self {
        let loaded = load_last_session(paths);
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
#[allow(
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;
    use ntd_settings::save_last_session;

    #[test]
    fn without_a_saved_session_the_defaults_are_used() {
        let dir = crate::test_support::test_paths();
        let service = ConfigService::load(&dir);
        assert!(service.corrections.is_empty());
        assert_eq!(
            service.config.active_area,
            MappingConfig::default().active_area
        );
        assert_eq!(
            service.config.run_at_startup,
            crate::startup::is_run_at_startup_registered()
        );
    }

    #[test]
    fn the_last_session_is_restored() {
        let dir = crate::test_support::test_paths();
        let mut saved = MappingConfig::default();
        saved.active_area.w = 77.0;
        save_last_session(&dir, &saved).unwrap();

        let service = ConfigService::load(&dir);
        assert_eq!(service.config.active_area.w, 77.0);
        assert!(service.corrections.is_empty());
    }

    #[test]
    fn a_damaged_session_is_repaired_and_the_repairs_are_reported() {
        let dir = crate::test_support::test_paths();
        let mut saved = MappingConfig::default();
        saved.active_area.w = -5.0;
        save_last_session(&dir, &saved).unwrap();

        let service = ConfigService::load(&dir);
        assert!(service.config.active_area.w > 0.0);
        assert_eq!(service.corrections.len(), 1);
    }
}
