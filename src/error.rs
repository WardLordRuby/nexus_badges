use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    io,
};

pub enum Error {
    Io(io::Error),
    SerdeJson(serde_json::Error),
    Reqwest(reqwest::Error),
    Missing(&'static str),
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Error::Reqwest(value)
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::SerdeJson(value)
    }
}

impl Error {
    fn msg(&self) -> Cow<'static, str> {
        match self {
            Error::Io(err) => Cow::Owned(err.to_string()),
            Error::Missing(msg) => Cow::Borrowed(*msg),
            Error::Reqwest(err) => Cow::Owned(err.to_string()),
            Error::SerdeJson(err) => Cow::Owned(err.to_string()),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => write!(f, "{err:?}"),
            Error::Missing(msg) => write!(f, "{msg}"),
            Error::Reqwest(err) => write!(f, "{err:?}"),
            Error::SerdeJson(err) => write!(f, "{err:?}"),
        }
    }
}
