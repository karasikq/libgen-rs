#[derive(Debug)]
pub enum Error {
    ReqwestError(reqwest::Error),
    UrlParseError(url::ParseError),
    Generic(String),
    Download(String),
    Mirror(String)
}

impl Error {
    pub fn new<T: Into<String>>(msg: T) -> Self {
        Self::Generic(msg.into())
    }

    pub fn download<T: Into<String>>(msg: T) -> Self {
        Self::Download(msg.into())
    }

    pub fn mirror<T: Into<String>>(msg: T) -> Self {
        Self::Download(msg.into())
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::ReqwestError(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Self::UrlParseError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Generic(err.to_string())
    }
}

impl From<&'static str> for Error {
    fn from(err: &'static str) -> Self {
        Self::Generic(err.to_string())
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self::Generic(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReqwestError(err) => write!(f, "Reqwest error. Reason: {}", err),
            Self::UrlParseError(err) => write!(f, "Failed to parse url. Reason: {}", err),
            Self::Generic(err) => write!(f, "Error: {}", err),
            Self::Download(err) => write!(f, "Download error: {}", err),
            Self::Mirror(err) => write!(f, "Mirror error: {}", err),
        }
    }
}

impl std::error::Error for Error {}
