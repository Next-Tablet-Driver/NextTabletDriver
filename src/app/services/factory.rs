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
        })
    }
}
