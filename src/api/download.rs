use {
    crate::api::{book::Book, mirrors::Mirror},
    bytes::Bytes,
    lazy_static::lazy_static,
    regex::bytes::Regex,
    reqwest::Client,
    url::Url,
};

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

fn capture<'a>(regex: &Regex, download_page: &'a Bytes) -> Option<&'a str> {
    regex
        .captures(download_page)
        .map(|c| std::str::from_utf8(c.get(0).unwrap().as_bytes()).unwrap())
}

impl Mirror {
    pub async fn download_book(
        &self,
        client: &Client,
        book: &Book,
    ) -> Result<reqwest::Response, &'static str> {
        let download_page_url_md5 = self
            .download_pattern
            .as_ref()
            .unwrap()
            .replace("{md5}", &book.md5);
        let download_page_url = Url::parse(&download_page_url_md5).unwrap();

        let content = client
            .get(download_page_url)
            .send()
            .await
            .or(Err("Couldn't connect to mirror"))?
            .bytes()
            .await
            .or(Err("Couldn't get mirror page"))?;

        match self.host_url.as_str() {
            "https://libgen.rocks/" | "http://libgen.lc/" => {
                self.download_book_from_ads(&content, client).await
            }
            "https://libgen.lol/" | "http://libgen.me/" => {
                self.download_book_from_lol(&content, client).await
            }
            _ => return Err("Couldn't find download url"),
        }
        .map_err(|_| "Download error")
    }

    async fn download_book_from_ads(
        &self,
        download_page: &Bytes,
        client: &Client,
    ) -> Result<reqwest::Response, &'static str> {
        let Some(key) = capture(&KEY_REGEX, download_page) else {
            return Err("Couldn't find download key");
        };
        self.download(client, key).await
    }

    async fn download_book_from_lol(
        &self,
        download_page: &Bytes,
        client: &Client,
    ) -> Result<reqwest::Response, &'static str> {
        let Some(key) = capture(&KEY_REGEX_LOL, download_page)
            .or_else(|| capture(&KEY_REGEX_LOL_CLOUDFLARE, download_page))
            .or_else(|| capture(&KEY_REGEX_LOL_IPFS, download_page))
        else {
            return Err("Couldn't find download key");
        };

        self.download(client, key).await
    }
}
