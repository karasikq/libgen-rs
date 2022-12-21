use bytes::Bytes;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::bytes::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

use crate::api::book::Book;
use crate::api::mirrors::Url as _Url;

use super::mirrors::LibgenMetadata;

lazy_static! {
    static ref HASH_REGEX: Regex = Regex::new(r"[A-Z0-9]{32}").unwrap();
    static ref JSON_QUERY: String =
        "id,title,author,filesize,extension,md5,year,language,pages,publisher,edition,coverurl,descr,timeadded,timelastmodified"
            .to_string();
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum SearchOption {
    #[serde(rename = "def")]
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

pub struct Search {
    pub query: String,
    pub max_results: u32,
    pub search_option: SearchOption,
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
    pub column: SearchOption,
}

impl SearchQuery {
    pub fn new(query: String, max_results: u32, search_option: SearchOption) -> Self {
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

        let (cover_url, search_url) = self.get_cover_and_search_urls()?;

        for hash in hashes.iter() {
            let mut search_url = Url::parse(search_url.as_str())
                .map_err(|e| format!("invalid search url: {}", e.to_string()))?;
            search_url
                .query_pairs_mut()
                .append_pair("ids", hash)
                .append_pair("fields", &JSON_QUERY);
            let content = match Self::request_content_as_bytes(&search_url.as_str(), client).await {
                Ok(v) => v,
                Err(_) => continue,
            };
            let mut book: Vec<Book> =
                match serde_json::from_str(std::str::from_utf8(&content).unwrap()) {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Couldn't parse json");
                        continue;
                    }
                };
            book.iter_mut().for_each(|b| {
                b.coverurl = cover_url.replace("{cover-url}", &b.coverurl);
            });
            parsed_books.append(&mut book);
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
    search_option: SearchOption,
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
            search_option: SearchOption::Default,
            search_url,
            download_url,
            libgen_metadata: libgen_metadata,
        }
    }

    pub fn max_results(mut self, max_results: u32) -> Self {
        self.max_results = max_results;
        self
    }

    pub fn search_option(mut self, search_option: SearchOption) -> Self {
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
    use crate::api::{mirrors::LibgenMetadata, search::SearchBuilder};

    #[tokio::test]
    async fn it_builds_correctly() {
        let metadata = LibgenMetadata::from_json_file(None).await.unwrap();
        let search = SearchBuilder::new(metadata.clone(), "test".to_string())
            .max_results(50)
            .search_option(super::SearchOption::Default)
            .build();
        assert_eq!(search.query, "test");
        assert_eq!(search.max_results, 50);
        assert_eq!(search.search_option, super::SearchOption::Default);
        assert_eq!(search.download_url, metadata.downloadable_urls[0]);
        assert_eq!(search.search_url, metadata.searchable_urls[0]);
    }

    #[tokio::test]
    async fn it_searches() {
        let metadata = LibgenMetadata::from_json_file(None).await.unwrap();
        let search = SearchBuilder::new(metadata, "rust zero to production".to_string())
            .max_results(15)
            .search_option(super::SearchOption::Default)
            .build()
            .search()
            .await;
        assert!(search.is_ok());
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
