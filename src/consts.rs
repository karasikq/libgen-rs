use lazy_static::*;

use crate::api::mirrors::Mirror;

lazy_static! {
    pub static ref MIRRORS: Vec<Mirror> = vec![
        Mirror {
            label: "libgen.is".to_string(),
            url: "http://libgen.is/".to_string(),
            search_url: Some("https://libgen.is/search.php".to_string()),
            json_search_url: Some("https://libgen.is/json.php".to_string()),
            download_url: None,
            cover_url: Some("http://libgen.is/covers/{cover-url}".to_string()),
        },
        Mirror {
            label: "libgen.rs".to_string(),
            url: "http://libgen.rs/".to_string(),
            search_url: Some("https://libgen.rs/search.php".to_string()),
            json_search_url: Some("https://libgen.rs/json.php".to_string()),
            download_url: None,
            cover_url: Some("http://libgen.rs/covers/{cover-url}".to_string()),
        },
        Mirror {
            label: "libgen.st".to_string(),
            url: "http://libgen.st/".to_string(),
            search_url: Some("https://libgen.st/search.php".to_string()),
            json_search_url: Some("https://libgen.st/json.php".to_string()),
            download_url: None,
            cover_url: Some("http://libgen.st/covers/{cover-url}".to_string()),
        },
        Mirror {
            label: "library.lol".to_string(),
            url: "http://libgen.lol/".to_string(),
            search_url: None,
            download_url: Some("http://library.lol/main/{md5}".to_string()),
            cover_url: Some("http://libgen.rs/covers/{cover-url}".to_string()),
            json_search_url: Some("http://libgen.rs/json.php".to_string()),
        },
        Mirror {
            label: "libgen.lc".to_string(),
            url: "http://libgen.lc/".to_string(),
            search_url: None,
            download_url: Some("http://libgen.lc/get.php?md5={md5}".to_string()),
            cover_url: Some("http://libgen.lc/covers/{cover-url}".to_string()),
            json_search_url: Some("http://libgen.ls/json.php".to_string()),
        },
        Mirror {
            label: "libgen.rocks".to_string(),
            url: "https://libgen.rocks/".to_string(),
            search_url: None,
            json_search_url: None,
            download_url: Some("https://libgen.rocks/ads.php?md5={md5}".to_string()),
            cover_url: None,
        },
        Mirror {
            label: "libgen.me".to_string(),
            url: "https://libgen.me/".to_string(),
            search_url: None,
            json_search_url: None,
            download_url: Some("https://libgen.me/book/{md5}".to_string()),
            cover_url: None,
        }
    ];
}
