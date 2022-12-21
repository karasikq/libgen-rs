use bytes::Bytes;
use futures_util::StreamExt;
use lazy_static::lazy_static;
use regex::bytes::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{cmp::min, fmt, fs::File, io::Write, path::PathBuf};
use url::Url;

use super::mirrors::LibgenMetadata;

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Book {
    pub id: String,
    pub title: String,
    pub author: String,
    pub filesize: String,
    pub year: String,
    pub language: String,
    pub pages: String,
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
        amount_downloaded_out: Option<&impl Fn(u64)>,
    ) -> Result<(), String> {
        let download_url = &libgen_metadata.downloadable_urls[0];

        let downloaded = self
            .download(client, &download_url.url, host_url)
            .await
            .map_err(|e| e.to_string())?;

        let total_size = downloaded.content_length().unwrap();
        let mut book_download_path = PathBuf::from(download_path);
        std::fs::create_dir_all(&book_download_path).unwrap();

        //  TODO: review the max path/file name length for windows and linux
        let mut book_title = match self.title.len() {
            0..=249 => self.title.clone(),
            _ => self.title[0..249].to_string(),
        };

        //  Windows doesn't allow colon in file names, thx Bill Gates
        book_title = book_title.replace(":", "");
        book_download_path.push(book_title);
        book_download_path.set_extension(&self.extension);

        println!(
            "book_download_path with extension: {}",
            book_download_path.as_os_str().to_str().unwrap()
        );
        let mut stream = downloaded.bytes_stream();
        let mut file = File::create(book_download_path).unwrap();

        let mut amount_downloaded: u64 = 0;
        let has_callback = amount_downloaded_out.as_ref().is_some();
        while let Some(item) = stream.next().await {
            let chunk = item.or(Err("Error while downloading file")).unwrap();
            file.write_all(&chunk).unwrap();
            let new = min(amount_downloaded + (chunk.len() as u64), total_size);

            amount_downloaded = new;

            if has_callback {
                amount_downloaded_out.as_ref().unwrap()(amount_downloaded);
            }
        }
        Ok(())
    }

    pub async fn download(
        &self,
        client: &Client,
        download_url_with_md5: &str,
        host_url: &str,
    ) -> Result<reqwest::Response, &'static str> {
        let download_page_url_md5 = download_url_with_md5.replace("{md5}", &self.md5);
        let download_page_url = Url::parse(&download_page_url_md5).unwrap();

        let content = client
            .get(download_page_url)
            .send()
            .await
            .or(Err("Couldn't connect to mirror"))?
            .bytes()
            .await
            .or(Err("Couldn't get mirror page"))?;

        //  TODO: add more info to the libgen metadata so we dont need to hardcode this stuff
        match host_url {
            "https://libgen.rocks/" => {
                match self.download_from_ads(&content, client, host_url).await {
                    Ok(b) => Ok(b),
                    Err(_e) => Err("Download error"),
                }
            }
            "http://libgen.lc/" => match self.download_from_ads(&content, client, host_url).await {
                Ok(b) => Ok(b),
                Err(_e) => Err("Download error"),
            },
            "http://libgen.lol/" => {
                match self.download_from_lol(&content, client, host_url).await {
                    Ok(b) => Ok(b),
                    Err(_e) => Err("Download error"),
                }
            }
            "http://libgen.me/" => match self.download_from_lol(&content, client, host_url).await {
                Ok(b) => Ok(b),
                Err(_e) => Err("Download error"),
            },
            &_ => Err("Couldn't find download url"),
        }
    }

    async fn download_from_ads(
        &self,
        download_page: &Bytes,
        client: &Client,
        host_url: &str,
    ) -> Result<reqwest::Response, &'static str> {
        let key = KEY_REGEX
            .captures(download_page)
            .map(|c| std::str::from_utf8(c.get(0).unwrap().as_bytes()).unwrap());
        if key.is_none() {
            return Err("Couldn't find download key");
        }

        let download_url = Url::parse(host_url).unwrap();
        let options = Url::options();
        let base_url = options.base_url(Some(&download_url));
        let download_url = base_url.parse(key.unwrap()).unwrap();
        client
            .get(download_url)
            .send()
            .await
            .or(Err("Couldn't connect to mirror"))
    }

    async fn download_from_lol(
        &self,
        download_page: &Bytes,
        client: &Client,
        host_url: &str,
    ) -> Result<reqwest::Response, &'static str> {
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
            return Err("Couldn't find download key");
        }

        let download_url = Url::parse(host_url).unwrap();
        let options = Url::options();
        let base_url = options.base_url(Some(&download_url));
        let download_url = base_url.parse(key.unwrap()).unwrap();
        client
            .get(download_url)
            .send()
            .await
            .or(Err("Couldn't connect to mirror"))
    }
}

impl fmt::Display for Book {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}
