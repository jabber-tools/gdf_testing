use std::error::Error as StdError;
use jmespath::JmespathError;
use std::fmt;
use std::result;
use serde_json;
use reqwest;
use jsonwebtoken;
use yaml_rust::scanner::ScanError;

#[derive(Debug)]
pub enum ErrorKind {
    GDFTokenRetrievalError,
    GDFInvocationError,
    HttpInvocationError(reqwest::Error),
    YamlParsingError(String),
    YamlLoadingError(ScanError),
    JsonParsingError(JmespathError),
    IOError(std::io::Error),
    JsonSerDeser(serde_json::error::Error),
    JWTCreation(jsonwebtoken::errors::Error),
    GenericError(String)
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ErrorKind::GDFTokenRetrievalError => write!(f, "GDFTokenRetrievalError"),
            ErrorKind::GDFInvocationError => write!(f, "GDFInvocationError"),
            ErrorKind::HttpInvocationError(err) => write!(f, "HttpInvocationError"),
            ErrorKind::YamlParsingError(err) => write!(f, "YamlParsingError"),
            ErrorKind::YamlLoadingError(err) => write!(f, "YamlLoadingError"),
            ErrorKind::JsonParsingError(err) => write!(f, "JsonParsingError"),
            ErrorKind::IOError(err) => write!(f, "IOError"),
            ErrorKind::JsonSerDeser(err) => write!(f, "JsonSerDeser"),
            ErrorKind::JWTCreation(err) => write!(f, "JWTCreation"),
            ErrorKind::GenericError(err) => write!(f, "GenericError666")
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub kind: Box<ErrorKind>,
    pub message: String,
    pub code: Option<String>
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
            ErrorKind::YamlParsingError(ref err) => None,
            ErrorKind::YamlLoadingError(ref err) => Some(err),
            ErrorKind::JsonParsingError(ref err) => Some(err),
            ErrorKind::IOError(ref err) => Some(err),
            ErrorKind::JsonSerDeser(ref err) => Some(err),
            ErrorKind::JWTCreation(ref err) => Some(err),
            ErrorKind::GenericError(ref err) => None,
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

impl From<JmespathError> for Error {
    fn from(error: JmespathError) -> Error {
        new_error_from(ErrorKind::JsonParsingError(error))
    }
}

impl From<String> for Error {
    fn from(error: String) -> Error {
        new_error_from(ErrorKind::GenericError(format!("GenericError: {}", error)))
    }
}

impl From<yaml_rust::scanner::ScanError> for Error {
    fn from(error: yaml_rust::scanner::ScanError) -> Error {
        new_error_from(ErrorKind::YamlLoadingError(error))
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