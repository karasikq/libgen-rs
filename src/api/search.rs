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

#[repr(u32)]
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

impl SearchOption {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "def",
            Self::Title => "title",
            Self::Author => "author",
            Self::Series => "series",
            Self::Publisher => "publisher",
            Self::Year => "year",
            Self::ISBN => "identifier",
            Self::Language => "language",
            Self::MD5 => "md5",
            Self::Tags => "tags",
            Self::Extension => "extension",
        }
    }
}

fn parse_hashes(content: Bytes) -> Vec<String> {
    let hashes: Vec<_> = HASH_REGEX
        .captures_iter(&content)
        .flat_map(|caps| {
            caps.get(0)
                .map(|x| std::str::from_utf8(x.as_bytes()).unwrap().to_string())
        })
        .collect();

    hashes.iter().unique().cloned().collect()
}

async fn get_content(url: &Url, client: &Client) -> Result<Bytes, reqwest::Error> {
    client.get(url.as_str()).send().await?.bytes().await
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
        let search_url = search_url
            .query_pairs_mut()
            .append_pair("req", &self.request)
            .append_pair("lg_topic", "libgen")
            .append_pair("res", &results.to_string())
            .append_pair("open", "0")
            .append_pair("view", "simple")
            .append_pair("phrase", "1")
            .append_pair("column", self.search_option.as_str())
            .finish();

        let content = match get_content(search_url, client).await {
            Ok(b) => b,
            Err(_) => return Err("Error getting content from page"),
        };
        let book_hashes = parse_hashes(content);
        Ok(self.get_books(&book_hashes, client).await)
    }

    async fn get_books(&self, hashes: &[String], client: &Client) -> Vec<Book> {
        let mut parsed_books: Vec<Book> = vec![];
        let cover_url = String::from(self.mirror.cover_pattern.as_ref().unwrap());

        for hash in hashes.iter() {
            let mut search_url = Url::parse(
                self.mirror
                    .sync_url
                    .as_ref()
                    .expect("Expected an Url")
                    .as_str(),
            )
            .unwrap();
            search_url
                .query_pairs_mut()
                .append_pair("ids", hash)
                .append_pair("fields", &JSON_QUERY);
            let Ok(content) = get_content(&search_url, client).await else {
                continue;
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
