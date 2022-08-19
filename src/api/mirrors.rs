use reqwest::Client;
use reqwest::StatusCode;
use serde_json::Value;
use url::Url;
use std::fmt;

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
}

impl Mirror {
    pub async fn check_connection(&self, client: &Client) -> Result<(), StatusCode> {
        let resp = client.get(self.host_url.as_str()).send();
        match resp.await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.status().unwrap()),
        }
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
    pub fn parse_mirrors(json: &str) -> MirrorList {
        let mut search_mirrors: Vec<Mirror> = Vec::new();
        let mut download_mirrors: Vec<Mirror> = Vec::new();

        let map: Value = serde_json::from_str(json).unwrap();
        map.as_object().unwrap().iter().for_each(|(_k, v)| {
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
            if let Some(..) = host_url {
                if search_url.is_some() {
                    search_mirrors.push(Mirror {
                        host_url: host_url.unwrap(),
                        search_url,
                        download_url,
                        download_pattern,
                        sync_url,
                    })
                } else if download_url.is_some() {
                    download_mirrors.push(Mirror {
                        host_url: host_url.unwrap(),
                        search_url,
                        download_url,
                        download_pattern,
                        sync_url,
                    })
                }
            }
        });
        MirrorList {
            search_mirrors,
            download_mirrors,
        }
    }

    pub async fn get_working_mirror(
        &self,
        mirror_type: MirrorType,
        client: &Client,
    ) -> Result<Mirror, &'static str> {
        if let MirrorType::Search = mirror_type {
            for mirror in self.search_mirrors.iter() {
                match mirror.check_connection(client).await {
                    Ok(_) => return Ok(mirror.clone()),
                    Err(_e) => continue,
                };
            }
        } else {
            for mirror in self.download_mirrors.iter() {
                match mirror.check_connection(client).await {
                    Ok(_) => return Ok(mirror.clone()),
                    Err(_e) => continue,
                };
            }
        }
        Err("Couldn't reach mirrors")
    }

    pub fn get(&self, mirror_type: MirrorType, index: usize) -> Result<Mirror, &'static str> {
        if let MirrorType::Search = mirror_type {
            return Ok(self.search_mirrors.get(index).unwrap().clone());
        } else {
            return Ok(self.download_mirrors.get(index).unwrap().clone());
        }
    }
}
