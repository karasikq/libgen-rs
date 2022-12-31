use std::sync::Arc;

use crate::api::book::Book;
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

use super::mirrors::Mirror;
use super::mirrors::SearchMirror;

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
    pub search_url: String,
    pub json_search_url: String,
    pub cover_url: String,
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
        let search_url_with_query = format!("{}?{}", self.search_url, query_string);
        tracing::debug!(search_url_with_query);
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
        let search_url = Arc::new(self.json_search_url.clone());
        let cover_url = Arc::new(self.cover_url.clone());
        let mut futures = FuturesUnordered::new();

        for hash in hashes {
            let search_url = search_url.clone();
            let cover_url = cover_url.clone();
            let future_book_data_as_json = async move {
                let mut search_url = Url::parse(search_url.as_str()).map_err(|e| e.to_string())?;
                search_url
                    .query_pairs_mut()
                    .append_pair("ids", hash)
                    .append_pair("fields", &JSON_QUERY);
                tracing::debug!("requesting json book data at: {:?}", search_url.as_str());
                let request_content = Self::request_content_as_bytes(&search_url.as_str(), client)
                    .await
                    .map_err(|e| e.to_string())?;

                let request_content_as_str =
                    std::str::from_utf8(&request_content).map_err(|e| e.to_string())?;
                let mut books = serde_json::from_str::<Vec<Book>>(request_content_as_str)
                    .map_err(|e| e.to_string())?;

                for book in books.iter_mut() {
                    book.coverurl = cover_url.replace("{cover-url}", &book.coverurl);
                }

                //  https://github.com/rust-lang/rust/issues/63502#issue-479823017
                Ok::<Vec<Book>, String>(books)
            };
            futures.push(future_book_data_as_json);

            //  TODO: use multiple search urls? it gets rate limited pretty quickly with 10 concurrent requests
            //  TODO: don't hardcode the max number of concurrent tasks
            if futures.len() == 5 {
                if let Some(future) = futures.next().await {
                    match future {
                        Ok(mut item) => parsed_books.append(&mut item),
                        Err(e) => tracing::error!("{}", e),
                    }
                }
            }
        }
        while let Some(future) = futures.next().await {
            match future {
                Ok(mut item) => parsed_books.append(&mut item),
                Err(e) => tracing::error!("{}", e),
            }
        }
        Ok(parsed_books)
    }
}

pub struct SearchBuilder {
    query: String,
    max_results: u32,
    search_option: SearchIn,
    search_url: String,
    json_search_url: String,
    cover_url: String,
}

impl SearchBuilder {
    pub fn new(
        query: String,
        search_url: String,
        cover_url: String,
        json_search_url: String,
    ) -> Self {
        Self {
            query,
            max_results: 25,
            search_option: SearchIn::Default,
            search_url,
            json_search_url,
            cover_url,
        }
    }

    pub fn from_mirror(query: String, mirror: &SearchMirror) -> Self {
        Self {
            query,
            max_results: 25,
            search_option: SearchIn::Default,
            search_url: mirror.search_url.to_owned(),
            json_search_url: mirror.json_search_url.to_owned(),
            cover_url: mirror.cover_url.to_owned(),
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

    pub fn build(self) -> Search {
        Search {
            query: self.query,
            max_results: self.max_results,
            search_option: self.search_option,
            search_url: self.search_url,
            json_search_url: self.json_search_url,
            cover_url: self.cover_url,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::api::{mirrors::MirrorList, search::SearchBuilder};

    #[test]
    fn it_builds_correctly() {
        let mirror_list = MirrorList::new();
        let selected_mirror = mirror_list.mirrors[0].clone();
        let search = SearchBuilder::new(
            "test".to_string(),
            selected_mirror.search_url.clone().unwrap(),
            selected_mirror.cover_url.unwrap(),
            selected_mirror.json_search_url.unwrap(),
        )
        .max_results(50)
        .search_option(super::SearchIn::Default)
        .build();
        assert_eq!(search.query, "test");
        assert_eq!(search.max_results, 50);
        assert_eq!(search.search_option, super::SearchIn::Default);
        assert_eq!(search.search_url, selected_mirror.search_url.unwrap());
    }

    #[tokio::test]
    async fn it_searches() {
        let mirror_list = MirrorList::new();
        let selected_mirror = mirror_list.mirrors[0].clone();
        let search = SearchBuilder::new(
            "rust zero to production".to_string(),
            selected_mirror.search_url.unwrap(),
            selected_mirror.cover_url.unwrap(),
            selected_mirror.json_search_url.unwrap(),
        )
        .build();
        let search_result = search.search().await;
        assert!(search_result.is_ok());
    }
}
