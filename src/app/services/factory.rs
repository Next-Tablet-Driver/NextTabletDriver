use std::sync::Arc;
use crate::engine::state::SharedState;
use crate::drivers::TabletData;
use crate::core::config::models::MappingConfig;
use std::sync::RwLock;
use std::sync::atomic::AtomicU32;

pub struct SharedStateFactory;

impl SharedStateFactory {
    pub fn create(config: MappingConfig, is_first_run: bool) -> Arc<SharedState> {
        Arc::new(SharedState {
            config: RwLock::new(config),
            config_version: AtomicU32::new(0),
            tablet_data: RwLock::new(TabletData::default()),
            device_state: RwLock::new(crate::engine::state::DeviceState::default()),
            is_first_run: RwLock::new(is_first_run),
            packet_count: AtomicU32::new(0),
            stats: RwLock::new(crate::drivers::DriverStats::default()),
            shutdown_requested: std::sync::atomic::AtomicBool::new(false),
        })
    }
}
