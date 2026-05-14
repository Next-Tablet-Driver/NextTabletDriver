pub mod github;
pub mod installer;
pub mod models;

pub use github::check_for_updates;
pub use installer::download_and_install;
pub use models::{Asset, Release, UpdateStatus, Version};
