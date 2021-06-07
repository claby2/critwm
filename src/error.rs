use std::{fmt, io};
use x11_dl::error as x11_error;

pub type CritResult<T> = Result<T, CritError>;

#[derive(Debug)]
pub enum CritError {
    Open(x11_error::OpenError),
    Io(io::Error),
    SerdeJson(serde_json::Error),
    Other(String),
}

impl fmt::Display for CritError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CritError::*;
        match self {
            Open(ref e) => e.fmt(f),
            Io(ref e) => e.fmt(f),
            SerdeJson(ref e) => e.fmt(f),
            Other(ref s) => write!(f, "{}", s),
        }
    }
}

impl From<x11_error::OpenError> for CritError {
    fn from(e: x11_error::OpenError) -> Self {
        Self::Open(e)
    }
}

impl From<io::Error> for CritError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for CritError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeJson(e)
    }
}
