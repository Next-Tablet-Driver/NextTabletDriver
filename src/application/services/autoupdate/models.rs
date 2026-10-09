use serde::{Deserialize, Serialize};

/// Represents a GitHub Release object returned from the API.
#[derive(Deserialize, Serialize, Clone)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    pub body: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// Represents an individual file (asset) attached to a GitHub Release.
#[derive(Deserialize, Serialize, Clone)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}
