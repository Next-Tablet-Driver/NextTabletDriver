/// Services that load the configuration, build the shared state, run the background threads and
/// talk to the network.
pub mod services;
/// State the interface reads: a snapshot per frame, the console view and the main window models.
pub mod state;
pub mod telemetry;
