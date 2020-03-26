use std::error::Error as StdError;
use std::fmt;
use std::result;
use serde_json;
use reqwest;
use jsonwebtoken;

#[derive(Debug)]
pub enum ErrorKind {
    GDFTokenRetrievalError,
    GDFInvocationError,
    HttpInvocationError(reqwest::Error),
    YamlParsingError,
    JsonParsingError,
    IOError(std::io::Error),
    JsonSerDeser(serde_json::error::Error),
    JWTCreation(jsonwebtoken::errors::Error)
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ErrorKind::GDFTokenRetrievalError => write!(f, "GDFTokenRetrievalError"),
            ErrorKind::GDFInvocationError => write!(f, "GDFInvocationError"),
            ErrorKind::HttpInvocationError(err) => write!(f, "HttpInvocationError"),
            ErrorKind::YamlParsingError => write!(f, "YamlParsingError"),
            ErrorKind::JsonParsingError => write!(f, "JsonParsingError"),
            ErrorKind::IOError(err) => write!(f, "IOError"),
            ErrorKind::JsonSerDeser(err) => write!(f, "JsonSerDeser"),
            ErrorKind::JWTCreation(err) => write!(f, "JWTCreation"),
        }
    }
}

#[derive(Debug)]
pub struct Error {
    kind: Box<ErrorKind>,
    message: String,
    code: Option<String>
}

pub type Result<T> = result::Result<T, Error>;

/// A crate private constructor for `Error`.
pub(crate) fn new_error_from(kind: ErrorKind) -> Error {
    let message = kind.to_string();
    Error {
        kind: Box::new(kind),
        message: message,
        code: None
    }
}

pub(crate) fn new_error(kind: ErrorKind, message: String, code: Option<String>) -> Error {
    Error {
        kind: Box::new(kind),
        message,
        code
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.code {
            Some(code) => write!(f, "Error occurred. Kind: {}, Code: {}, Message: {}", &self.kind, code, &self.message),
            _ => write!(f, "Error occurred. Kind: {}, Code: N/A, Message: {}", &self.kind, &self.message)
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self.kind {
            ErrorKind::GDFInvocationError => None,
            ErrorKind::GDFTokenRetrievalError => None,
            ErrorKind::HttpInvocationError(ref err) => Some(err),
            ErrorKind::YamlParsingError => None,
            ErrorKind::JsonParsingError => None,
            ErrorKind::IOError(ref err) => Some(err),
            ErrorKind::JsonSerDeser(ref err) => Some(err),
            ErrorKind::JWTCreation(ref err) => Some(err),
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(error: ErrorKind) -> Error {
        new_error_from(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Error {
        new_error_from(ErrorKind::IOError(error))
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(error: serde_json::error::Error) -> Error {
        new_error_from(ErrorKind::JsonSerDeser(error))
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Error {
        new_error_from(ErrorKind::HttpInvocationError(error))
    }
}

impl From<jsonwebtoken::errors::Error> for Error {
    fn from(error: jsonwebtoken::errors::Error) -> Error {
        new_error_from(ErrorKind::JWTCreation(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_rendering() {
        assert_eq!(
            "Error occurred. Kind: GDFTokenRetrievalError, Code: N/A, Message: GDFTokenRetrievalError",
            Error::from(ErrorKind::GDFTokenRetrievalError).to_string()
        );

        assert_eq!(
            "Error occurred. Kind: GDFInvocationError, Code: ERR-001, Message: ooops",
            new_error(ErrorKind::GDFInvocationError, "ooops".to_owned(), Some("ERR-001".to_owned())).to_string()
        );        
    }
}