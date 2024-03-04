use std::fmt::Display;

use crate::error::Error;
use regex::bytes::Regex;
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
    pub download_regexes: Vec<String>,
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
    pub donwload_regexes: Vec<Regex>,
}

pub struct MirrorList {
    pub mirrors: Vec<Mirror>,
    pub download_mirrors: Vec<DownloadMirror>,
    pub search_mirrors: Vec<SearchMirror>,
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

impl MirrorList {
    /// From a valid json file containing an array of mirrors (check mirrors.json)
    pub fn from_json_file(file: &str) -> Result<Self, Error> {
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
    pub fn from_json_str(json: &str) -> Result<Self, Error> {
        let mirrors: Vec<Mirror> = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let (search_mirrors, download_mirrors) = Self::get_search_and_download_mirrors(&mirrors)?;
        let list = Self {
            mirrors,
            search_mirrors,
            download_mirrors,
        };
        Ok(list)
    }

    pub fn from_json_slice(json: &[u8]) -> Result<Self, Error> {
        let mirrors: Vec<Mirror> = serde_json::from_slice(json).map_err(|e| e.to_string())?;
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
    ) -> Result<(Vec<SearchMirror>, Vec<DownloadMirror>), Error> {
        let mut search_mirrors = vec![];
        let mut download_mirrors = vec![];
        for mirror in mirrors {
            if let Some(download_url) = &mirror.download_url {
                download_mirrors.push(DownloadMirror {
                    label: mirror.label.clone(),
                    host_url: mirror.url.clone(),
                    download_url: download_url.to_owned(),
                    donwload_regexes: mirror
                        .download_regexes
                        .iter()
                        .map(|r| -> Result<Regex, Error> {
                            Regex::new(r).map_err(|e| {
                                Error::Mirror(format!("Cannot parse download regex. Reason: {}", e))
                            })
                        })
                        .collect::<Result<Vec<_>, Error>>()?,
                });
            }

            if let (Some(search_url), Some(json_search_url), Some(cover_url)) = (
                mirror.search_url.as_ref(),
                mirror.json_search_url.as_ref(),
                mirror.cover_url.as_ref(),
            ) {
                search_mirrors.push(SearchMirror {
                    label: mirror.label.clone(),
                    search_url: search_url.clone(),
                    json_search_url: json_search_url.clone(),
                    cover_url: cover_url.clone(),
                });
            }
        }
        if search_mirrors.is_empty() && download_mirrors.is_empty() {
            Err(Error::mirror(
                "No search and download mirrors was found in the provided list",
            ))
        } else {
            Ok((search_mirrors, download_mirrors))
        }
    }

    pub fn get_search_mirror(&self, index: usize) -> Result<SearchMirror, Error> {
        match self.search_mirrors.get(index) {
            Some(mirror) => Ok(mirror.clone()),
            None => Err(Error::Generic(format!(
                "Cannot get mirror with index {}",
                index
            ))),
        }
    }

    pub fn get_download_mirror(&self, index: usize) -> Result<DownloadMirror, Error> {
        match self.download_mirrors.get(index) {
            Some(mirror) => Ok(mirror.clone()),
            None => Err(Error::Generic(format!(
                "Cannot get mirror with index {}",
                index
            ))),
        }
    }
}

impl Default for MirrorList {
    fn default() -> Self {
        Self::from_json_str(include_str!("../../resources/mirrors.json")).unwrap()
    }
}

impl Display for SearchMirror {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

impl Display for DownloadMirror {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[cfg(test)]
mod tests {
    use crate::mirrors::MirrorList;

    #[test]
    fn default_json() {
        let _ = MirrorList::default();
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
