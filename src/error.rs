pub enum LibgenApiError {
    ReqwestError(reqwest::Error),
    UrlParseError(url::ParseError),
    Generic(String),
}

impl LibgenApiError {
    pub fn new<T: Into<String>>(msg: T) -> Self {
        Self::Generic(msg.into())
    }
}

impl ToString for LibgenApiError {
    fn to_string(&self) -> String {
        match self {
            Self::ReqwestError(err) => err.to_string(),
            Self::UrlParseError(err) => err.to_string(),
            Self::Generic(err) => err.to_string(),
        }
    }
}

impl From<reqwest::Error> for LibgenApiError {
    fn from(err: reqwest::Error) -> Self {
        Self::ReqwestError(err)
    }
}

impl From<url::ParseError> for LibgenApiError {
    fn from(err: url::ParseError) -> Self {
        Self::UrlParseError(err)
    }
}

impl From<std::io::Error> for LibgenApiError {
    fn from(err: std::io::Error) -> Self {
        Self::Generic(err.to_string())
    }
}
