use crate::consts;

use super::mirrors::{Mirror, Url};

#[derive(Clone)]
pub struct LibgenMetadata {
    pub mirrors: Vec<Mirror>,
    pub searchable_urls: Vec<Url>,
    pub downloadable_urls: Vec<Url>,
}

impl LibgenMetadata {
    /// From the default static mirrors
    pub fn new() -> Result<Self, String> {
        let mirrors = consts::MIRRORS.to_vec();
        Self::from_mirror_vec(mirrors)
    }

    pub fn from_mirror_vec(mirrors: Vec<Mirror>) -> Result<Self, String> {
        let (downloadable_urls, searchable_urls) =
            LibgenMetadata::try_get_down_and_search_urls(&mirrors)?;
        Ok(Self {
            mirrors,
            searchable_urls,
            downloadable_urls,
        })
    }

    /// From a valid json file containing an array of mirrors (check mirrors.json)
    pub async fn from_json_file(file: &str) -> Result<LibgenMetadata, String> {
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
        LibgenMetadata::from_json_str(parsed_file_content.as_str())
    }

    /// From a valid json string containing an array of mirrors
    pub fn from_json_str(json: &str) -> Result<LibgenMetadata, String> {
        let mirrors: Vec<Mirror> = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let (downloadable_urls, searchable_urls) =
            LibgenMetadata::try_get_down_and_search_urls(&mirrors)?;
        Ok(LibgenMetadata {
            mirrors,
            downloadable_urls,
            searchable_urls,
        })
    }

    /// This function will try to extract searchable and downloadable urls from the provided mirrors.   
    ///
    /// If no searchable or downloadable urls are found, it will return an error.
    ///
    /// The first element of the tuple is the downloadable urls, the second is the searchable urls.
    fn try_get_down_and_search_urls(mirrors: &Vec<Mirror>) -> Result<(Vec<Url>, Vec<Url>), String> {
        let mut downloadable_urls: Vec<Url> = Vec::new();
        let mut searchable_urls: Vec<Url> = Vec::new();
        for mirror in mirrors {
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

        if searchable_urls.is_empty() {
            return Err("No searchable urls found in the provided json".to_string());
        }
        if downloadable_urls.is_empty() {
            return Err("No downloadable urls found in the provided json".to_string());
        }
        Ok((downloadable_urls, searchable_urls))
    }
}
