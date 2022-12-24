use bytes::Bytes;
use futures_util::StreamExt;
use lazy_static::lazy_static;
use regex::bytes::Regex;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::{cmp::min, fmt, fs::File, io::Write, path::PathBuf};
use url::Url;

use crate::api::metadata::LibgenMetadata;

lazy_static! {
    static ref KEY_REGEX: Regex = Regex::new(r"get\.php\?md5=\w{32}&key=\w{16}").unwrap();
    static ref KEY_REGEX_LOL: Regex =
        Regex::new(r"http://62\.182\.86\.140/main/\d{7}/\w{32}/.+?(gz|pdf|rar|djvu|epub|chm)")
            .unwrap();
    static ref KEY_REGEX_LOL_CLOUDFLARE: Regex = Regex::new(
        r"https://cloudflare-ipfs\.com/ipfs/\w{62}\?filename=.+?(gz|pdf|rar|djvu|epub|chm)"
    )
    .unwrap();
    static ref KEY_REGEX_LOL_IPFS: Regex =
        Regex::new(r"https://ipfs\.io/ipfs/\w{62}\?filename=.+?(gz|pdf|rar|djvu|epub|chm)")
            .unwrap();
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Book {
    pub id: String,
    pub title: String,
    pub author: String,
    pub filesize: String,
    pub year: String,
    pub language: String,
    pub pages: String,
    pub descr: Option<String>,
    pub timeadded: String,
    pub timelastmodified: String,
    pub publisher: String,
    pub edition: String,
    pub extension: String,
    pub md5: String,
    pub coverurl: String,
}

impl Book {
    //  TODO: allow user to specify download mirror
    pub async fn download_to_path(
        &self,
        client: &Client,
        host_url: &str,
        libgen_metadata: &LibgenMetadata,
        download_path: &str,
        progress_callback: Option<impl FnOnce(u64) -> () + Copy>,
    ) -> Result<(), String> {
        let download_url = &libgen_metadata.downloadable_urls[0];

        let downloaded = self
            .download(client, &download_url.url, host_url)
            .await
            .map_err(|e| e.to_string())?;

        let total_size = downloaded
            .content_length()
            .ok_or("Couldn't extract the content length from the downloaded book")?;
        let mut book_download_path = PathBuf::from(download_path);
        std::fs::create_dir_all(&book_download_path).map_err(|e| e.to_string())?;

        //  TODO: review the max path/file name length for windows and linux
        let mut book_title = match self.title.len() {
            0..=249 => self.title.clone(),
            _ => self.title[0..249].to_string(),
        };

        //  Windows doesn't allow colon in file names, thx Bill Gates
        book_title = book_title.replace(":", "");
        book_download_path.push(book_title);
        book_download_path.set_extension(&self.extension);

        let mut stream = downloaded.bytes_stream();
        let mut file = File::create(book_download_path).map_err(|e| e.to_string())?;

        let mut amount_downloaded: u64 = 0;
        while let Some(item) = stream.next().await {
            let chunk = item
                .or(Err("Error while downloading file"))
                .map_err(|e| e.to_string())?;
            file.write_all(&chunk).map_err(|e| e.to_string())?;
            let new = min(amount_downloaded + (chunk.len() as u64), total_size);

            amount_downloaded = new;
            if let Some(callback) = progress_callback {
                callback(amount_downloaded);
            }
        }
        Ok(())
    }

    pub async fn download(
        &self,
        client: &Client,
        download_url_with_md5: &str,
        host_url: &str,
    ) -> Result<reqwest::Response, String> {
        let download_page_url_md5 = download_url_with_md5.replace("{md5}", &self.md5);
        let download_page_url = Url::parse(&download_page_url_md5).map_err(|e| e.to_string())?;

        let content = client
            .get(download_page_url)
            .send()
            .await
            .or(Err("Couldn't connect to mirror"))?
            .bytes()
            .await
            .or(Err("Couldn't get mirror page"))?;

        //  I don't really know why this happens, but the compiler will complain if i send and
        //  await the requests inside the functions below while in a multithreaded environment or in a tokio::spawn
        //  TODO: add more info to the libgen metadata so we dont need to hardcode these urls
        return match host_url {
            "https://libgen.rocks/" | "http://libgen.lc/" => {
                Ok(Self::download_from_ads(&content, client, host_url)
                    .await?
                    .send()
                    .await
                    .map_err(|e| e.to_string())?)
            }
            "http://libgen.lol/" | "http://libgen.me/" => {
                Ok(Self::download_from_lol(&content, client, host_url)
                    .await?
                    .send()
                    .await
                    .map_err(|e| e.to_string())?)
            }
            _ => Err("Couldn't find download url".to_string()),
        };
    }

    async fn download_from_ads(
        download_page: &Bytes,
        client: &Client,
        host_url: &str,
    ) -> Result<RequestBuilder, String> {
        let Some(key) = KEY_REGEX
            .captures(download_page)
            .map(|c| std::str::from_utf8(c.get(0).unwrap().as_bytes()).unwrap())
            else {
                return Err("Couldn't find download key".to_string());
            };
        let download_url = Url::parse(host_url).map_err(|e| e.to_string())?;
        let options = Url::options();
        let base_url = options.base_url(Some(&download_url));
        let download_url = base_url.parse(key).map_err(|e| e.to_string())?;
        Ok(client.get(download_url))
    }

    async fn download_from_lol(
        download_page: &Bytes,
        client: &Client,
        host_url: &str,
    ) -> Result<RequestBuilder, String> {
        let mut key = KEY_REGEX_LOL
            .captures(download_page)
            .map(|c| std::str::from_utf8(c.get(0).unwrap().as_bytes()).unwrap());
        if key.is_none() {
            key = KEY_REGEX_LOL_CLOUDFLARE
                .captures(download_page)
                .map(|c| std::str::from_utf8(c.get(0).unwrap().as_bytes()).unwrap());
        }
        if key.is_none() {
            key = KEY_REGEX_LOL_IPFS
                .captures(download_page)
                .map(|c| std::str::from_utf8(c.get(0).unwrap().as_bytes()).unwrap());
        }
        if key.is_none() {
            return Err("Couldn't find download key".to_string());
        }

        let download_url = Url::parse(host_url).map_err(|e| e.to_string())?;
        let options = Url::options();
        let base_url = options.base_url(Some(&download_url));
        let download_url = base_url.parse(key.unwrap()).map_err(|e| e.to_string())?;
        Ok(client.get(download_url))
    }
}

impl fmt::Display for Book {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}
