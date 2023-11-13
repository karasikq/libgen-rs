use {
    crate::api::{book::Book, mirrors::Mirror},
    bytes::Bytes,
    itertools::Itertools,
    lazy_static::lazy_static,
    regex::bytes::Regex,
    reqwest::Client,
    std::cmp::Ordering,
    url::Url,
};

lazy_static! {
    static ref HASH_REGEX: Regex = Regex::new(r"[A-Z0-9]{32}").unwrap();
    static ref JSON_QUERY: String =
        "id,title,author,filesize,extension,md5,year,language,pages,publisher,edition,coverurl"
            .to_string();
}

#[repr(usize)]
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

impl TryFrom<usize> for SearchOption {
    type Error = &'static str;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Default,
            1 => Self::Title,
            2 => Self::Author,
            3 => Self::Series,
            4 => Self::Publisher,
            5 => Self::Year,
            6 => Self::ISBN,
            7 => Self::Language,
            8 => Self::MD5,
            9 => Self::Tags,
            10 => Self::Extension,
            _ => return Err("Unknown option"),
        })
    }
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

        let Ok(content) = get_content(search_url, client).await else {
            return Err("Error getting content from page");
        };
        let book_hashes = parse_hashes(content);
        Ok(self.mirror.get_books(&book_hashes, client).await)
    }
}

impl Mirror {
    async fn get_books(&self, hashes: &[String], client: &Client) -> Vec<Book> {
        let mut parsed_books: Vec<Book> = vec![];
        let cover_url = String::from(self.cover_pattern.as_ref().unwrap());

        for hash in hashes.iter() {
            let mut search_url =
                Url::parse(self.sync_url.as_ref().expect("Expected an Url").as_str()).unwrap();
            search_url
                .query_pairs_mut()
                .append_pair("ids", hash)
                .append_pair("fields", &JSON_QUERY);
            let Ok(content) = get_content(&search_url, client).await else {
                continue;
            };

            let Ok(mut book) =
                serde_json::from_str::<Vec<Book>>(std::str::from_utf8(&content).unwrap())
            else {
                println!("Couldn't parse json");
                continue;
            };
            book.iter_mut().for_each(|b| {
                if self.cover_pattern.is_some() {
                    b.coverurl = cover_url.replace("{cover-url}", &b.coverurl);
                }
            });
            parsed_books.append(&mut book);
        }
        parsed_books
    }

    pub async fn download(
        &self,
        client: &Client,
        key: &str,
    ) -> Result<reqwest::Response, &'static str> {
        let download_url = Url::parse(self.host_url.as_ref()).unwrap();
        let download_url = Url::options()
            .base_url(Some(&download_url))
            .parse(key)
            .unwrap();

        client
            .get(download_url)
            .send()
            .await
            .or(Err("Couldn't connect to mirror"))
    }
}
