use serde::{Deserialize, Serialize};

/// Represents a GitHub Release object returned from the API.
#[derive(Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Release {
    /// Git tag of the release, for example `v1.26.1708.00`.
    pub tag_name: String,
    /// Display name of the release.
    #[serde(default)]
    pub name: Option<String>,
    /// Release notes, in Markdown.
    pub body: Option<String>,
    /// Publication date, as GitHub formats it (ISO 8601).
    #[serde(default)]
    pub published_at: Option<String>,
    /// Files attached to the release.
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// Represents an individual file (asset) attached to a GitHub Release.
#[derive(Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Asset {
    /// File name of the asset.
    pub name: String,
    /// URL the asset can be downloaded from.
    pub browser_download_url: String,
}
