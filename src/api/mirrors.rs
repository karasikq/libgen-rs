use crate::consts;
use reqwest::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Mirror {
    pub label: String,
    pub url: String,
    pub search_url: Option<String>,
    pub json_search_url: Option<String>,
    pub download_url: Option<String>,
    pub cover_url: Option<String>,
}

#[derive(Clone)]
pub struct SearchMirror {
    pub label: String,
    pub search_url: String,
    pub json_search_url: String,
    pub cover_url: String,
}

#[derive(Clone)]
pub struct DownloadMirror {
    pub label: String,
    pub host_url: String,
    pub download_url: String,
}

impl Mirror {
    pub async fn check_connection(&self, client: &Client) -> Result<(), StatusCode> {
        client
            .get(self.url.as_str())
            .send()
            .await
            .map(|_| ())
            .map_err(|e| e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
    }
}

pub struct MirrorList {
    pub mirrors: Vec<Mirror>,
    pub download_mirrors: Vec<DownloadMirror>,
    pub search_mirrors: Vec<SearchMirror>,
}

impl MirrorList {
    /// From the default static mirrors
    pub fn new() -> Self {
        Self::from_static_mirrors()
    }

    pub fn from_static_mirrors() -> Self {
        let mirrors = consts::MIRRORS.to_vec();
        let (search_mirrors, download_mirrors) =
            Self::get_search_and_download_mirrors(&mirrors).unwrap();
        Self {
            mirrors,
            download_mirrors,
            search_mirrors,
        }
    }

    /// From a valid json file containing an array of mirrors (check mirrors.json)
    pub fn from_json_file(file: &str) -> Result<Self, String> {
        let file_content = std::fs::read(file).map_err(|e| {
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
        Self::from_json_str(parsed_file_content.as_str())
    }

    /// From a valid json string containing an array of mirrors
    pub fn from_json_str(json: &str) -> Result<Self, String> {
        let mirrors: Vec<Mirror> = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let (search_mirrors, download_mirrors) = Self::get_search_and_download_mirrors(&mirrors)?;
        let list = Self {
            mirrors,
            search_mirrors,
            download_mirrors,
        };
        Ok(list)
    }

    fn get_search_and_download_mirrors(
        mirrors: &Vec<Mirror>,
    ) -> Result<(Vec<SearchMirror>, Vec<DownloadMirror>), String> {
        let mut search_mirrors = vec![];
        let mut download_mirrors = vec![];
        for mirror in mirrors {
            if let Some(download_url) = &mirror.download_url {
                download_mirrors.push(DownloadMirror {
                    label: mirror.label.clone(),
                    host_url: mirror.url.clone(),
                    download_url: download_url.to_owned(),
                });
            }
            match (
                mirror.search_url.as_ref(),
                mirror.json_search_url.as_ref(),
                mirror.cover_url.as_ref(),
            ) {
                (Some(search_url), Some(json_search_url), Some(cover_url)) => {
                    search_mirrors.push(SearchMirror {
                        label: mirror.label.clone(),
                        search_url: search_url.clone(),
                        json_search_url: json_search_url.clone(),
                        cover_url: cover_url.clone(),
                    });
                }
                _ => (),
            }
        }
        if search_mirrors.is_empty() {
            return Err("No search mirror was found in the provided list".to_string());
        }
        if download_mirrors.is_empty() {
            return Err("No downloaded mirror was found in the provided list".to_string());
        }
        Ok((search_mirrors, download_mirrors))
    }
}

mod test {
    use super::MirrorList;
    #[tokio::test]
    async fn create_list_if_everything_ok() {
        let json_str = "[{\"label\":\"libgen.is\",\"url\":\"http://libgen.is/\",\"search_url\":\"https://libgen.is/search.php\",\"download_url\":\"http://libgen.is/get.php\",\"json_search_url\":\"http://libgen.is/json.php\",\"cover_url\":\"http://libgen.is/covers/{cover-url}\"}]";
        assert!(MirrorList::from_json_str(json_str).is_ok())
    }

    #[test]
    fn errors_if_not_a_single_downloadable_url() {
        let json_str_with_search = "[{\"label\":\"libgen.is\",\"url\":\"http://libgen.is/\",\"search_url\":\"https://libgen.is/search.php\"}]";
        assert!(MirrorList::from_json_str(json_str_with_search).is_err())
    }

    #[test]
    fn errors_if_no_search_url() {
        let json_str_with_download = "[{\"label\":\"library.lol\",\"url\":\"http://libgen.lol/\",\"download_url\":\"http://library.lol/main/{md5}\"}]";
        assert!(MirrorList::from_json_str(json_str_with_download).is_err())
    }
}
