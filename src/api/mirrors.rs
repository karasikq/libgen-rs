use reqwest::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Mirror {
    pub host_label: String,
    pub host_url: String,
    pub search_url: Option<String>,
    pub non_fiction_download_url: Option<String>,
    pub non_fiction_cover_url: Option<String>,
    pub non_fiction_sync_url: Option<String>,
    pub download_pattern: Option<String>,
    pub cover_pattern: Option<String>,
}

impl Mirror {
    pub async fn check_connection(&self, client: &Client) -> Result<(), StatusCode> {
        client
            .get(self.host_url.as_str())
            .send()
            .await
            .map(|_| ())
            .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Url {
    pub host_label: String,
    pub url: String,
}
