use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::fmt;
use url::Url;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MirrorType {
    Search,
    Download,
}

#[derive(Clone)]
pub struct Mirror {
    pub host_url: Url,
    pub search_url: Option<Url>,
    pub download_url: Option<Url>,
    pub download_pattern: Option<String>,
    pub sync_url: Option<Url>,
    pub cover_pattern: Option<String>,
}

impl Mirror {
    pub async fn check_connection(&self, client: &Client) -> Result<(), StatusCode> {
        let resp = client.get(self.host_url.as_str()).send().await;

        resp.map(|_| ()).map_err(|e| e.status().unwrap())
    }
}

impl fmt::Display for Mirror {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.host_url)
    }
}

pub struct MirrorList {
    pub search_mirrors: Vec<Mirror>,
    pub download_mirrors: Vec<Mirror>,
}

impl MirrorList {
    pub fn parse(path: &str) -> Self {
        let mut config_path = dirs::config_dir().unwrap();
        config_path.push(path);
        let json =
            std::str::from_utf8(&std::fs::read(config_path).expect("Couldn't read config file"))
                .unwrap()
                .to_owned();
        Self::parse_mirrors(&json)
    }

    pub fn parse_mirrors(json: &str) -> Self {
        let mut search_mirrors: Vec<Mirror> = vec![];
        let mut download_mirrors: Vec<Mirror> = vec![];

        let map: Value = serde_json::from_str(json).unwrap();
        map.as_object().unwrap().iter().for_each(|(_, v)| {
            let search_url = v
                .get("SearchUrl")
                .map(|v| Url::parse(v.as_str().unwrap()).unwrap());
            let host_url = v
                .get("Host")
                .map(|v| Url::parse(v.as_str().unwrap()).unwrap());
            let download_url = v
                .get("NonFictionDownloadUrl")
                .map(|v| Url::parse(&v.as_str().unwrap().replace("{md5}", "")).unwrap());
            let download_pattern = v
                .get("NonFictionDownloadUrl")
                .map(|v| v.as_str().unwrap().to_owned());
            let sync_url = v
                .get("NonFictionSynchronizationUrl")
                .map(|v| Url::parse(v.as_str().unwrap()).unwrap());
            let cover_pattern = v
                .get("NonFictionCoverUrl")
                .map(|v| String::from(v.as_str().unwrap()));
            if let Some(..) = host_url {
                let mirror = Mirror {
                    host_url: host_url.unwrap(),
                    search_url,
                    download_url,
                    download_pattern,
                    sync_url,
                    cover_pattern,
                };
                if mirror.search_url.is_some() {
                    search_mirrors.push(mirror);
                } else if mirror.download_url.is_some() {
                    download_mirrors.push(mirror);
                }
            }
        });
        Self {
            search_mirrors,
            download_mirrors,
        }
    }

    pub fn mirrors(&self, mirror_type: MirrorType) -> &[Mirror] {
        if MirrorType::Search == mirror_type {
            &self.search_mirrors
        } else {
            &self.download_mirrors
        }
    }

    pub async fn get_working_mirror(
        &self,
        mirror_type: MirrorType,
        client: &Client,
    ) -> Result<Mirror, &'static str> {
        let mirrors = self.mirrors(mirror_type);
        for mirror in mirrors.iter() {
            if mirror.check_connection(client).await.is_ok() {
                return Ok(mirror.clone());
            };
        }
        Err("Couldn't reach mirrors")
    }

    pub fn get(&self, mirror_type: MirrorType, index: usize) -> Result<Mirror, &'static str> {
        Ok(self.mirrors(mirror_type).get(index).unwrap().clone())
    }
}
