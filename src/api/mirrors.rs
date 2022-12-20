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

pub struct LibgenMetadata {
    pub mirrors: Vec<Mirror>,
    pub searchable_urls: Vec<Url>,
    pub downloadable_urls: Vec<Url>,
}

impl LibgenMetadata {
    pub async fn from_json_file(file: Option<&str>) -> Result<LibgenMetadata, String> {
        let file_content = std::fs::read(file.unwrap_or("mirrors.json")).map_err(|e| {
            format!(
                "Couldn't read the provided json file: {}",
                e.to_string().to_lowercase()
            )
        })?;
        let parsed_file_content = std::str::from_utf8(&file_content)
            .map_err(|e| {
                format!(
                    "Couldn't parse the provided json file to string format: {}",
                    e.to_string().to_lowercase()
                )
            })?
            .to_owned();
        LibgenMetadata::from_json_str(parsed_file_content.as_str())
    }

    pub fn from_json_str(json: &str) -> Result<LibgenMetadata, String> {
        let mirrors: Vec<Mirror> = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let mut downloadable_urls: Vec<Url> = Vec::new();
        let mut searchable_urls: Vec<Url> = Vec::new();
        for mirror in &mirrors {
            if mirror.non_fiction_download_url.is_some() {
                downloadable_urls.push(Url {
                    host_label: mirror.host_label.clone(),
                    url: mirror.non_fiction_download_url.clone().unwrap(),
                });
            }
            if mirror.search_url.is_some() {
                searchable_urls.push(Url {
                    host_label: mirror.host_label.clone(),
                    url: mirror.search_url.clone().unwrap(),
                });
            }
        }

        Ok(LibgenMetadata {
            mirrors,
            downloadable_urls,
            searchable_urls,
        })
    }
}

//  need better naming, maybe use the url crate
pub struct Url {
    pub host_label: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use crate::api::mirrors::LibgenMetadata;

    static VALID_MIRROR_JSON: &str = "[{\"host_label\":\"libgen.me\",\"host_url\":\"https://libgen.me/\",\"non_fiction_download_url\":\"https://libgen.me/book/{md5}\"}]";

    #[tokio::test]
    async fn errors_on_unexisting_file() {
        assert!(
            LibgenMetadata::from_json_file(Some("thisfiledoesnotexist.json"))
                .await
                .is_err()
        );
    }

    #[test]
    fn parses_correct_json() {
        assert!(LibgenMetadata::from_json_str(VALID_MIRROR_JSON).is_ok())
    }
}
