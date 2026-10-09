use crate::core::config::models::MappingConfig;
use crate::engine::state::{ConfigState, LifecycleState, PipelineState, SharedState};
use std::sync::Arc;
use std::sync::RwLock;

pub struct SharedStateFactory;

impl SharedStateFactory {
    #[must_use]
    pub fn create(config: MappingConfig, is_first_run: bool) -> Arc<SharedState> {
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
            plugins: Arc::new(crate::engine::plugins::PluginManager::new()),
        })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn the_state_starts_from_the_given_config_and_first_run_flag() {
        let mut config = MappingConfig::default();
        config.active_area.w = 33.0;
        let shared = SharedStateFactory::create(config, true);
        assert_eq!(shared.config.mapping.read().unwrap().active_area.w, 33.0);
        assert!(*shared.lifecycle.is_first_run.read().unwrap());
        assert_eq!(shared.config.version.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn everything_else_starts_idle() {
        let shared = SharedStateFactory::create(MappingConfig::default(), false);
        assert!(!*shared.lifecycle.is_first_run.read().unwrap());
        assert_eq!(shared.pipeline.packet_count.load(Ordering::Relaxed), 0);
        assert_eq!(shared.device.read().unwrap().vid, 0);
        assert!(!shared.lifecycle.shutdown_requested.load(Ordering::Relaxed));
        assert!(shared.lifecycle.is_visible.load(Ordering::Relaxed));
    }

    #[test]
    fn each_call_makes_an_independent_state() {
        let a = SharedStateFactory::create(MappingConfig::default(), false);
        let b = SharedStateFactory::create(MappingConfig::default(), false);
        a.pipeline.packet_count.store(9, Ordering::Relaxed);
        assert_eq!(b.pipeline.packet_count.load(Ordering::Relaxed), 0);
    }
}
