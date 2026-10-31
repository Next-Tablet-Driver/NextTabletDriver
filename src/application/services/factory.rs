use crate::engine::state::{ConfigState, LifecycleState, PipelineState, SharedState};
use ntd_core::config::models::MappingConfig;
use ntd_settings::Paths;
use std::sync::Arc;
use std::sync::RwLock;

/// Builds the shared state that the engine and the interface use.
pub struct SharedStateFactory;

impl SharedStateFactory {
    /// Creates the shared state from `config`; `is_first_run` records whether this is the first run
    /// of the application. The plugins are the ones of the plugins folder of `paths`.
    #[must_use]
    pub fn create(config: MappingConfig, is_first_run: bool, paths: &Paths) -> Arc<SharedState> {
        Arc::new(SharedState {
            config: ConfigState {
                mapping: RwLock::new(config),
                ..ConfigState::new()
            },
            pipeline: PipelineState::new(),
            device: RwLock::new(crate::engine::state::DeviceState::default()),
            lifecycle: LifecycleState {
                is_first_run: RwLock::new(is_first_run),
                ..LifecycleState::new()
            },
            plugins: Arc::new(ntd_plugin_host::PluginManager::new(paths)),
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn the_state_starts_from_the_given_config_and_first_run_flag() {
        let mut config = MappingConfig::default();
        config.active_area.w = 33.0;
        let paths = crate::test_support::test_paths();
        let shared = SharedStateFactory::create(config, true, &paths);
        assert_eq!(shared.config.mapping.read().unwrap().active_area.w, 33.0);
        assert!(*shared.lifecycle.is_first_run.read().unwrap());
        assert_eq!(shared.config.version.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn everything_else_starts_idle() {
        let paths = crate::test_support::test_paths();
        let shared = SharedStateFactory::create(MappingConfig::default(), false, &paths);
        assert!(!*shared.lifecycle.is_first_run.read().unwrap());
        assert_eq!(shared.pipeline.packet_count.load(Ordering::Relaxed), 0);
        assert_eq!(shared.device.read().unwrap().vid, 0);
        assert!(!shared.lifecycle.shutdown_requested.load(Ordering::Relaxed));
        assert!(shared.lifecycle.is_visible.load(Ordering::Relaxed));
    }

    #[test]
    fn each_call_makes_an_independent_state() {
        let paths = crate::test_support::test_paths();
        let a = SharedStateFactory::create(MappingConfig::default(), false, &paths);
        let b = SharedStateFactory::create(MappingConfig::default(), false, &paths);
        a.pipeline.packet_count.store(9, Ordering::Relaxed);
        assert_eq!(b.pipeline.packet_count.load(Ordering::Relaxed), 0);
    }
}
