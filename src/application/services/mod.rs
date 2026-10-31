/// Checks GitHub for newer releases.
pub mod autoupdate;
/// Loading of the configuration the application starts with.
pub mod config;
/// Construction of the shared state.
pub mod factory;
/// Background threads of the desktop application.
pub mod supervisor;
pub mod websocket;

pub use config::ConfigService;
pub use factory::SharedStateFactory;
pub use supervisor::ThreadSupervisor;
