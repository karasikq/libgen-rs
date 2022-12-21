use std::sync::Arc;

use crate::api::book::Book;
use crate::api::mirrors::Url as _Url;
use bytes::Bytes;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::bytes::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use strum::{Display, EnumIter, EnumString};
use url::Url;

use crate::api::metadata::LibgenMetadata;

lazy_static! {
    static ref HASH_REGEX: Regex = Regex::new(r"[A-Z0-9]{32}").unwrap();
    static ref JSON_QUERY: String =
        "id,title,author,filesize,extension,md5,year,language,pages,publisher,edition,coverurl,descr,timeadded,timelastmodified"
            .to_string();
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone, EnumIter, EnumString, Display)]
pub enum SearchIn {
    #[serde(rename = "def")]
    #[strum(to_string = "Default (All fields)")]
    Default,
    #[serde(rename = "title")]
    Title,
    #[serde(rename = "author")]
    Author,
    #[serde(rename = "series")]
    Series,
    #[serde(rename = "publisher")]
    Publisher,
    #[serde(rename = "year")]
    Year,
    #[serde(rename = "identifier")]
    ISBN,
    #[serde(rename = "language")]
    Language,
    #[serde(rename = "md5")]
    MD5,
    #[serde(rename = "tags")]
    Tags,
    #[serde(rename = "extension")]
    Extension,
}

impl Default for SearchIn {
    fn default() -> Self {
        Self::Default
    }
}

//  TODO: add offset support
//  TODO: add sorting support
pub struct Search {
    pub query: String,
    pub max_results: u32,
    pub search_option: SearchIn,
    pub search_url: _Url,
    pub download_url: _Url,
    libgen_metadata: LibgenMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct SearchQuery {
    pub req: String,
    pub lg_topic: String,
    pub res: String,
    pub open: String,
    pub view: String,
    pub phrase: String,
    pub column: SearchIn,
}

impl SearchQuery {
    pub fn new(query: String, max_results: u32, search_option: SearchIn) -> Self {
        Self {
            req: query,
            lg_topic: "libgen".to_string(),
            res: max_results.to_string(),
            open: "0".to_string(),
            view: "simple".to_string(),
            phrase: "1".to_string(),
            column: search_option,
        }
    }
}

impl Search {
    pub async fn search(&self) -> Result<Vec<Book>, String> {
        let query_string = self.generate_query_string()?;
        let search_url_with_query = format!("{}?{}", self.search_url.url, query_string);
        let reqwest_client = Client::new();
        let response = Self::request_content_as_bytes(&search_url_with_query, &reqwest_client)
            .await
            .map_err(|e| e.to_string())?;
        let book_hashes = Self::parse_hashes(&response);
        let books = self.get_books(&book_hashes, &reqwest_client).await?;
        Ok(books)
    }

    fn generate_query_string(&self) -> Result<String, String> {
        serde_qs::to_string(&SearchQuery::new(
            self.query.clone(),
            self.max_results,
            self.search_option.clone(),
        ))
        .map_err(|e| e.to_string())
    }

    async fn request_content_as_bytes(url: &str, client: &Client) -> Result<Bytes, reqwest::Error> {
        client.get(url).send().await?.bytes().await
    }

    fn parse_hashes(content: &Bytes) -> Vec<String> {
        let mut hashes: Vec<String> = Vec::new();
        for caps in HASH_REGEX.captures_iter(&content) {
            let capture = match caps.get(0) {
                Some(c) => c,
                None => continue,
            };
            hashes.push(std::str::from_utf8(capture.as_bytes()).unwrap().to_string());
        }
        hashes.iter().unique().cloned().collect::<Vec<_>>()
    }

    async fn get_books(&self, hashes: &[String], client: &Client) -> Result<Vec<Book>, String> {
        let mut parsed_books: Vec<Book> = Vec::new();

        //  TODO: get multiple urls from each kind so we have less chance to get rate limited
        let (cover_url, search_url) = self.get_cover_and_search_urls()?;
        let search_url = Arc::new(search_url);
        let cover_url = Arc::new(cover_url);
        let mut futs = FuturesUnordered::new();

        //  this can be refactored
        //  TODO: remove the myriad of unwraps from below
        for hash in hashes.iter() {
            let search_url = search_url.clone();
            let cover_url = cover_url.clone();
            let fut = async move {
                let mut result: Vec<Book> = Vec::new();
                let mut search_url = Url::parse(search_url.as_str())
                    .map_err(|e| format!("invalid search url: {}", e.to_string()))
                    .unwrap();
                search_url
                    .query_pairs_mut()
                    .append_pair("ids", hash)
                    .append_pair("fields", &JSON_QUERY);
                let content: Result<Bytes, reqwest::Error> =
                    match Self::request_content_as_bytes(&search_url.as_str(), client).await {
                        Ok(c) => Ok(c),
                        Err(_) => {
                            return result;
                        }
                    };

                let mut books: Result<Vec<Book>, String> = match serde_json::from_str::<Vec<Book>>(
                    std::str::from_utf8(&content.unwrap()).unwrap(),
                ) {
                    Ok(v) => Ok(v),
                    Err(_) => {
                        return result;
                    }
                };

                books = books.map(|mut v| {
                    v.iter_mut().for_each(|b| {
                        b.coverurl = cover_url.replace("{cover-url}", &b.coverurl);
                    });
                    v
                });
                result = books.unwrap();
                result
            };
            futs.push(fut);

            //  TODO: don't hardcode this
            if futs.len() == 5 {
                parsed_books.append(&mut futs.next().await.unwrap());
            }
        }
        while let Some(item) = futs.next().await {
            parsed_books.append(&mut item.clone());
        }
        Ok(parsed_books)
    }

    pub fn get_cover_and_search_urls(&self) -> Result<(String, String), String> {
        //  TODO: do this computation earlier so we don't need to do this on every search

        let cover_url = self
            .libgen_metadata
            .mirrors
            .clone()
            .into_iter()
            .find(|el| el.non_fiction_cover_url.is_some())
            .ok_or("Couldn't find a mirror with a cover url")?
            .non_fiction_cover_url
            .unwrap();

        let search_url = self
            .libgen_metadata
            .mirrors
            .clone()
            .into_iter()
            .find(|el| el.non_fiction_sync_url.is_some())
            .ok_or("Couldn't find a mirror with a search url")?
            .non_fiction_sync_url
            .unwrap();

        Ok((cover_url, search_url))
    }
}

pub struct SearchBuilder {
    query: String,
    max_results: u32,
    search_option: SearchIn,
    download_url: _Url,
    search_url: _Url,
    libgen_metadata: LibgenMetadata,
}

impl SearchBuilder {
    pub fn new(libgen_metadata: LibgenMetadata, query: String) -> Self {
        let search_url = libgen_metadata.searchable_urls[0].clone();
        let download_url = libgen_metadata.downloadable_urls[0].clone();
        Self {
            query,
            max_results: 25,
            search_option: SearchIn::Default,
            search_url,
            download_url,
            libgen_metadata: libgen_metadata,
        }
    }

    pub fn max_results(mut self, max_results: u32) -> Self {
        self.max_results = max_results;
        self
    }

    pub fn search_option(mut self, search_option: SearchIn) -> Self {
        self.search_option = search_option;
        self
    }

    pub fn download_url(mut self, download_url: _Url) -> Self {
        self.download_url = download_url;
        self
    }

    pub fn search_url(mut self, search_url: _Url) -> Self {
        self.search_url = search_url;
        self
    }

    pub fn build(self) -> Search {
        Search {
            query: self.query,
            max_results: self.max_results,
            search_option: self.search_option,
            search_url: self.search_url,
            download_url: self.download_url,
            libgen_metadata: self.libgen_metadata,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::api::{metadata::LibgenMetadata, search::SearchBuilder};

    #[tokio::test]
    async fn it_builds_correctly() {
        let metadata = LibgenMetadata::from_json_file("mirrors.json")
            .await
            .unwrap();
        let search = SearchBuilder::new(metadata.clone(), "test".to_string())
            .max_results(50)
            .search_option(super::SearchIn::Default)
            .build();
        assert_eq!(search.query, "test");
        assert_eq!(search.max_results, 50);
        assert_eq!(search.search_option, super::SearchIn::Default);
        assert_eq!(search.download_url, metadata.downloadable_urls[0]);
        assert_eq!(search.search_url, metadata.searchable_urls[0]);
    }

    #[tokio::test]
    async fn it_searches() {
        let metadata = LibgenMetadata::new().unwrap();
        let search = SearchBuilder::new(metadata, "rust".to_string())
            .max_results(10)
            .search_option(super::SearchIn::Default)
            .build();
        let search_result = search.search().await;
        assert!(search_result.is_ok());
    }

    #[test]
    fn errors_if_not_a_single_downlodable_url() {
        let json_str_with_search = "[{\"host_label\":\"libgen.is\",\"host_url\":\"http://libgen.is/\",\"search_url\":\"https://libgen.is/search.php\"}]";
        assert!(LibgenMetadata::from_json_str(json_str_with_search).is_err())
    }

    #[test]
    fn errors_if_not_a_single_searchable_url() {
        let json_str_with_download = "[{\"host_label\":\"library.lol\",\"host_url\":\"http://libgen.lol/\",\"non_fiction_download_url\":\"http://library.lol/main/{md5}\"}]";
        assert!(LibgenMetadata::from_json_str(json_str_with_download).is_err())
    }

    #[test]
    fn creates_metadata_if_downloads_and_searches_are_present() {
        let json_str = "[{\"host_label\":\"libgen.is\",\"host_url\":\"http://libgen.is/\",\"search_url\":\"https://libgen.is/search.php\",\"non_fiction_download_url\":\"http://libgen.is/get.php\"}]";
        assert!(LibgenMetadata::from_json_str(json_str).is_ok())
    }

    #[test]
    fn errors_if_no_sync_url() {
        let json_str = "[{\"host_label\":\"library.lol\",\"host_url\":\"http://libgen.lol/\",\"search_url\":\"https://libgen.st/search.php\",\"non_fiction_cover_url\":\"http://libgen.st/covers/{cover-url}\",\"non_fiction_download_url\":\"http://library.lol/main/{md5}\"}]";
        let metadata = LibgenMetadata::from_json_str(json_str).unwrap();
        let search = SearchBuilder::new(metadata, "rust zero to production".to_string()).build();
        assert!(search.get_cover_and_search_urls().is_err())
    }

    #[test]
    fn errors_if_no_cover_url() {
        let json_str = "[{\"host_label\":\"library.lol\",\"host_url\":\"http://libgen.lol/\",\"search_url\":\"https://libgen.st/search.php\",\"non_fiction_download_url\":\"http://library.lol/main/{md5}\",\"non_fiction_sync_url\":\"http://libgen.rs/json.php\"}]";
        let metadata = LibgenMetadata::from_json_str(json_str).unwrap();
        let search = SearchBuilder::new(metadata, "rust zero to production".to_string()).build();
        assert!(search.get_cover_and_search_urls().is_err())
    }
}
