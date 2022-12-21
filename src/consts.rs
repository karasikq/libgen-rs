use lazy_static::*;

use crate::api::mirrors::Mirror;

lazy_static! {
    pub static ref MIRRORS: Vec<Mirror> = vec![
        Mirror {
            host_label: "libgen.is".to_string(),
            host_url: "http://libgen.is/".to_string(),
            search_url: Some("https://libgen.is/search.php".to_string()),
            non_fiction_download_url: None,
            non_fiction_cover_url: Some("http://libgen.is/covers/{cover-url}".to_string()),
            non_fiction_sync_url: Some("http://libgen.is/json.php".to_string()),
            download_pattern: None,
            cover_pattern: None
        },
        Mirror {
            host_label: "libgen.rs".to_string(),
            host_url: "http://libgen.rs/".to_string(),
            search_url: Some("https://libgen.rs/search.php".to_string()),
            non_fiction_download_url: None,
            non_fiction_cover_url: Some("http://libgen.rs/covers/{cover-url}".to_string()),
            non_fiction_sync_url: Some("http://libgen.rs/json.php".to_string()),
            download_pattern: None,
            cover_pattern: None
        },
        Mirror {
            host_label: "libgen.st".to_string(),
            host_url: "http://libgen.st/".to_string(),
            search_url: Some("https://libgen.st/search.php".to_string()),
            non_fiction_download_url: None,
            non_fiction_cover_url: Some("http://libgen.st/covers/{cover-url}".to_string()),
            non_fiction_sync_url: Some("http://libgen.st/json.php".to_string()),
            download_pattern: None,
            cover_pattern: None
        },
        Mirror {
            host_label: "library.lol".to_string(),
            host_url: "http://libgen.lol/".to_string(),
            search_url: None,
            non_fiction_download_url: Some("http://library.lol/main/{md5}".to_string()),
            non_fiction_cover_url: Some("http://libgen.rs/covers/{cover-url}".to_string()),
            non_fiction_sync_url: Some("http://libgen.rs/json.php".to_string()),
            download_pattern: None,
            cover_pattern: None
        },
        Mirror {
            host_label: "libgen.lc".to_string(),
            host_url: "http://libgen.lc/".to_string(),
            search_url: None,
            non_fiction_download_url: Some("http://libgen.lc/get.php?md5={md5}".to_string()),
            non_fiction_cover_url: Some("http://libgen.lc/covers/{cover-url}".to_string()),
            non_fiction_sync_url: Some("http://libgen.ls/json.php".to_string()),
            download_pattern: None,
            cover_pattern: None
        },
        Mirror {
            host_label: "libgen.rocks".to_string(),
            host_url: "https://libgen.rocks/".to_string(),
            search_url: None,
            non_fiction_download_url: Some("https://libgen.rocks/ads.php?md5={md5}".to_string()),
            non_fiction_cover_url: None,
            non_fiction_sync_url: None,
            download_pattern: None,
            cover_pattern: None
        },
        Mirror {
            host_label: "libgen.me".to_string(),
            host_url: "https://libgen.me/".to_string(),
            search_url: None,
            non_fiction_download_url: Some("https://libgen.me/book/{md5}".to_string()),
            non_fiction_cover_url: None,
            non_fiction_sync_url: None,
            download_pattern: None,
            cover_pattern: None
        }
    ];
}
