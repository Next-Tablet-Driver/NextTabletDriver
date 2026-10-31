/// Queries the GitHub API for the releases of the project.
pub mod github;
/// Data returned by the GitHub releases API.
pub mod models;

pub use github::fetch_releases;
pub use models::{Asset, Release};
