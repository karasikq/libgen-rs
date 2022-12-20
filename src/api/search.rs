use bytes::Bytes;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::bytes::Regex;
use reqwest::Client;
use std::cmp::Ordering;
use url::Url;

use crate::api::book::Book;
use crate::api::mirrors::Mirror;

lazy_static! {
    static ref HASH_REGEX: Regex = Regex::new(r"[A-Z0-9]{32}").unwrap();
    static ref JSON_QUERY: String =
        "id,title,author,filesize,extension,md5,year,language,pages,publisher,edition,coverurl"
            .to_string();
}

pub enum SearchOption {
    Default,
    Title,
    Author,
    Series,
    Publisher,
    Year,
    ISBN,
    Language,
    MD5,
    Tags,
    Extension,
}

pub struct Search {
    pub mirror: Mirror,
    pub request: String,
    pub results: u32,
    pub search_option: SearchOption,
}

impl Search {
    pub async fn search(&self, client: &Client) -> Result<Vec<Book>, &'static str> {
        let results = match self.results.cmp(&50) {
            Ordering::Less => 25,
            Ordering::Equal => 50,
            Ordering::Greater => 100,
        };

        let mut search_url = Url::parse(
            self.mirror
                .search_url
                .as_ref()
                .expect("Mirror search url is invalid")
                .as_str(),
        )
        .unwrap();
        let mut search_query = search_url.query_pairs_mut();
        search_query
            .append_pair("req", &self.request)
            .append_pair("lg_topic", "libgen")
            .append_pair("res", &results.to_string())
            .append_pair("open", "0")
            .append_pair("view", "simple")
            .append_pair("phrase", "1");
        match self.search_option {
            SearchOption::Default => search_query.append_pair("column", "def"),
            SearchOption::Title => search_query.append_pair("column", "title"),
            SearchOption::Author => search_query.append_pair("column", "author"),
            SearchOption::Series => search_query.append_pair("column", "series"),
            SearchOption::Publisher => search_query.append_pair("column", "publisher"),
            SearchOption::Year => search_query.append_pair("column", "year"),
            SearchOption::ISBN => search_query.append_pair("column", "identifier"),
            SearchOption::Language => search_query.append_pair("column", "language"),
            SearchOption::MD5 => search_query.append_pair("column", "md5"),
            SearchOption::Tags => search_query.append_pair("column", "tags"),
            SearchOption::Extension => search_query.append_pair("column", "extension"),
        };
        let search_url = search_query.finish();
        let content = match Self::get_content(search_url, client).await {
            Ok(b) => b,
            Err(_) => return Err("Error getting content from page"),
        };
        let book_hashes = Self::parse_hashes(content);
        Ok(Self::get_books(self, &book_hashes, client).await)
    }

    async fn get_content(url: &Url, client: &Client) -> Result<Bytes, reqwest::Error> {
        client.get(url.as_str()).send().await?.bytes().await
    }

    fn parse_hashes(content: Bytes) -> Vec<String> {
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

    async fn get_books(&self, hashes: &[String], client: &Client) -> Vec<Book> {
        let mut parsed_books: Vec<Book> = Vec::new();
        let cover_url = String::from(self.mirror.cover_pattern.as_ref().unwrap());

        for hash in hashes.iter() {
            let mut search_url = Url::parse(
                self.mirror
                    .non_fiction_sync_url
                    .as_ref()
                    .expect("Expected an Url")
                    .as_str(),
            )
            .unwrap();
            search_url
                .query_pairs_mut()
                .append_pair("ids", hash)
                .append_pair("fields", &JSON_QUERY);
            let content = match Self::get_content(&search_url, client).await {
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
                if self.mirror.cover_pattern.is_some() {
                    b.coverurl = cover_url.replace("{cover-url}", &b.coverurl);
                }
            });
            parsed_books.append(&mut book);
        }
        parsed_books
    }
}
