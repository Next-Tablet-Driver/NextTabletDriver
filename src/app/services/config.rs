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
