use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{cmp::min, fmt::Display, fs::File, io::Write, path::PathBuf};
use url::Url;

use crate::{error::Error, mirrors::DownloadMirror};

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
    pub async fn download_to_path<P>(
        &self,
        client: Option<&reqwest::Client>,
        download_mirror: DownloadMirror,
        download_path: P,
        progress_callback: Option<impl FnOnce(u64, u64) + Copy>,
    ) -> Result<(), Error>
    where
        P: Into<PathBuf>,
    {
        let downloaded = self
            .download(client.unwrap_or(&reqwest::Client::new()), &download_mirror)
            .await?;

        let total_size = downloaded
            .content_length()
            .ok_or(Error::download("Couldn't extract the content length"))?;

        let mut book_download_path = download_path.into();
        tracing::debug!("Book download path: {:?}", book_download_path);

        std::fs::create_dir_all(&book_download_path)?;
        tracing::debug!("Created the directory for the book download path if it didn't exist.");

        //  TODO: write regex to check naming on Windows & UNIX
        let book_title = match self.title.len() {
            0..=249 => &self.title,
            _ => &self.title[0..249],
        };

        book_download_path.push(book_title);
        book_download_path.set_extension(&self.extension);

        let mut stream = downloaded.bytes_stream();
        let mut file = File::create(book_download_path)?;

        let mut amount_downloaded: u64 = 0;
        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|e| {
                Error::download(format!(
                    "Couldn't get next chunk. Downloaded: {}B\nReason: {}",
                    amount_downloaded, e,
                ))
            })?;
            file.write_all(&chunk)?;
            let new = min(amount_downloaded + (chunk.len() as u64), total_size);

            amount_downloaded = new;
            if let Some(callback) = progress_callback {
                callback(amount_downloaded, total_size);
            }
        }
        Ok(())
    }

    pub async fn download(
        &self,
        client: &Client,
        mirror: &DownloadMirror,
    ) -> Result<reqwest::Response, Error> {
        let download_url_with_md5 = mirror.download_url.replace("{md5}", &self.md5);
        let download_url = Url::parse(&download_url_with_md5)?;

        let content = client.get(download_url).send().await?.bytes().await?;
        let url = Self::parse_page(&content, mirror)?;

        client.get(url).send().await.map_err(Error::ReqwestError)
    }

    fn parse_page(page: &Bytes, mirror: &DownloadMirror) -> Result<Url, Error> {
        for regex in mirror.donwload_regexes.iter() {
            if let Some(key) = regex
                .captures(page)
                .map(|c| std::str::from_utf8(c.get(0).unwrap().as_bytes()).unwrap())
            {
                let download_url = Url::parse(&mirror.download_url)?;
                let options = Url::options();
                let base_url = options.base_url(Some(&download_url));
                let download_url = base_url.parse(key)?;
                return Ok(download_url);
            }
        }
        Err(Error::new("Couldn't find download key"))
    }
}

impl Display for Book {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}
