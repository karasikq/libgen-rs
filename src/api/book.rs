use bytes::Bytes;
use futures_util::StreamExt;
use lazy_static::lazy_static;
use regex::bytes::Regex;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::{cmp::min, fs::File, io::Write, path::PathBuf};
use url::Url;

use crate::error::LibgenApiError;

use super::mirrors::DownloadMirror;

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
    pub async fn download_to_path(
        &self,
        client: Option<&reqwest::Client>,
        download_mirror: DownloadMirror,
        download_path: &str,
        progress_callback: Option<impl FnOnce(u64) -> () + Copy>,
    ) -> Result<(), LibgenApiError> {
        let downloaded = self
            .download(
                client.unwrap_or(&reqwest::Client::new()),
                &download_mirror.download_url,
                &download_mirror.host_url,
            )
            .await?;

        let total_size = downloaded.content_length().ok_or(LibgenApiError::new(
            "Couldn't extract the content length from the downloaded book",
        ))?;
        let mut book_download_path = PathBuf::from(download_path);
        tracing::debug!("Book download path: {:?}", book_download_path);

        std::fs::create_dir_all(&book_download_path)?;
        tracing::debug!("Created the directory for the book download path if it didn't exist.");

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
        let mut file = File::create(book_download_path)?;

        let mut amount_downloaded: u64 = 0;
        while let Some(item) = stream.next().await {
            let chunk = item.or(Err(LibgenApiError::new("Error while downloading file")))?;
            file.write_all(&chunk)?;
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
    ) -> Result<reqwest::Response, LibgenApiError> {
        let download_url_with_md5 = download_url_with_md5.replace("{md5}", &self.md5);
        let download_url = Url::parse(&download_url_with_md5)?;

        let content = client.get(download_url).send().await?.bytes().await?;

        //  I don't really know why this happens, but the compiler will complain if i send and
        //  await the requests inside the functions below while in a multithreaded environment or in a tokio::spawn
        //  TODO: add more info to the libgen metadata so we dont need to hardcode these urls
        return match host_url {
            "https://libgen.rocks/" | "http://libgen.lc/" => {
                Ok(Self::download_from_ads(&content, client, host_url)
                    .await?
                    .send()
                    .await?)
            }
            "http://libgen.lol/" | "http://libgen.me/" => {
                Ok(Self::download_from_lol(&content, client, host_url)
                    .await?
                    .send()
                    .await?)
            }
            _ => Err(LibgenApiError::new("Couldn't find download url")),
        };
    }

    async fn download_from_ads(
        download_page: &Bytes,
        client: &Client,
        url: &str,
    ) -> Result<RequestBuilder, LibgenApiError> {
        let Some(key) = KEY_REGEX
            .captures(download_page)
            .map(|c| std::str::from_utf8(c.get(0).unwrap().as_bytes()).unwrap())
            else {
                return Err(LibgenApiError::new("Couldn't find download key"));
            };
        let download_url = Url::parse(url)?;
        let options = Url::options();
        let base_url = options.base_url(Some(&download_url));
        let download_url = base_url.parse(key)?;
        Ok(client.get(download_url))
    }

    async fn download_from_lol(
        download_page: &Bytes,
        client: &Client,
        url: &str,
    ) -> Result<RequestBuilder, LibgenApiError> {
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
            return Err(LibgenApiError::new("Couldn't find download key"));
        }

        let download_url = Url::parse(url)?;
        let options = Url::options();
        let base_url = options.base_url(Some(&download_url));
        let download_url = base_url.parse(key.unwrap())?;
        Ok(client.get(download_url))
    }
}
