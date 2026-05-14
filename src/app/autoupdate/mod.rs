pub mod models;
pub mod github;
pub mod installer;

pub use models::{Release, Asset, UpdateStatus, Version};
pub use github::check_for_updates;
pub use installer::download_and_install;
