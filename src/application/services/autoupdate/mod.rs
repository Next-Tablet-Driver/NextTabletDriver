pub mod github;
pub mod models;

pub use github::fetch_releases;
pub use models::{Asset, Release};
