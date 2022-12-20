use serde::{Deserialize, Serialize};
use std::fmt;

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

impl fmt::Display for Book {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}
