use std::fmt;
use x11_dl::error as x11_error;

pub type CritResult<T> = Result<T, CritError>;

#[derive(Debug)]
pub enum CritError {
    Open(x11_error::OpenError),
    Other(String),
}

impl fmt::Display for CritError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CritError::*;
        match self {
            Open(ref e) => e.fmt(f),
            Other(ref s) => write!(f, "{}", s),
        }
    }
}

impl From<x11_error::OpenError> for CritError {
    fn from(e: x11_error::OpenError) -> Self {
        Self::Open(e)
    }
}
