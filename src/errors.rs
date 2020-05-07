use std::error::Error as StdError;
use jmespath::JmespathError;
use std::fmt;
use std::result;
use serde_json;
use reqwest;
use jsonwebtoken;
use yaml_rust::scanner::ScanError;
use reqwest::header::InvalidHeaderValue;
use std::sync::mpsc::SendError;
use crate::yaml_parser::Test;
use serde::{Serialize, Deserialize};

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
    GenericError(String),
    InvalidHeaderValueError(InvalidHeaderValue),
    InvalidTestAssertionEvaluation,
    InvalidTestAssertionResponseCheckEvaluation,
    ChannelSendError(SendError<Test>)
}

//default is required if we want to skip ErrorKind for serialization/deserialization, see #[serde(skip)] below
impl Default for ErrorKind {
    fn default() -> Self { ErrorKind::GenericError(String::from("N/A")) }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ErrorKind::GDFTokenRetrievalError => write!(f, "GDFTokenRetrievalError"),
            ErrorKind::GDFInvocationError => write!(f, "GDFInvocationError"),
            ErrorKind::HttpInvocationError(_) => write!(f, "HttpInvocationError"),
            ErrorKind::YamlParsingError(_) => write!(f, "YamlParsingError"),
            ErrorKind::YamlLoadingError(_) => write!(f, "YamlLoadingError"),
            ErrorKind::JsonParsingError(_) => write!(f, "JsonParsingError"),
            ErrorKind::IOError(_) => write!(f, "IOError"),
            ErrorKind::JsonSerDeser(_) => write!(f, "JsonSerDeser"),
            ErrorKind::JWTCreation(_) => write!(f, "JWTCreation"),
            ErrorKind::GenericError(_) => write!(f, "GenericError"),
            ErrorKind::InvalidHeaderValueError(_) => write!(f, "InvalidHeaderValueError"),
            ErrorKind::InvalidTestAssertionEvaluation => write!(f, "InvalidTestAssertionEvaluation"),
            ErrorKind::InvalidTestAssertionResponseCheckEvaluation => write!(f, "InvalidTestAssertionResponseCheckEvaluation"),
            ErrorKind::ChannelSendError(_) => write!(f, "ChannelSendError"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    #[serde(skip)]
    pub kind: Box<ErrorKind>,
    pub message: String,
    pub code: Option<String>,
    pub backend_response: Option<String>,
}

impl Clone for Error {
    fn clone(&self) -> Error {
        Error {
            // some errors (e.g. ScanError) do not implement clone
            // easiest way to clone is to use GenericError and hence erase original kind
            // this is OK for us since we will be displaying messages only
            // we are cloning only when sending test results back to test suite executor
            // so this dirty workraround will not affect other errors like YAML parsing error
            // during loading test suite definition etc.
            kind: Box::new(ErrorKind::GenericError(String::from(""))),
            message: self.message.clone(),
            code: self.code.clone(),
            backend_response: self.backend_response.clone()
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

/// A crate private constructor for `Error`.
pub(crate) fn new_error_from(kind: ErrorKind) -> Error {
    let message = kind.to_string();
    Error {
        kind: Box::new(kind),
        message: message,
        code: None,
        backend_response: None // used to capture Google DialogFlow or VAP response for final error report. 
                               //User must see what went wrong while evaluating response from DialogFlow/VAP
    }
}

pub(crate) fn new_error(kind: ErrorKind, message: String, code: Option<String>) -> Error {
    Error {
        kind: Box::new(kind),
        message,
        code,
        backend_response: None
    }
}

pub(crate) fn new_service_call_error(kind: ErrorKind, message: String, code: Option<String>, backend_response: Option<String>) -> Error {
    Error {
        kind: Box::new(kind),
        message,
        code,
        backend_response
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
            ErrorKind::YamlParsingError(ref _err) => None,
            ErrorKind::YamlLoadingError(ref err) => Some(err),
            ErrorKind::JsonParsingError(ref err) => Some(err),
            ErrorKind::IOError(ref err) => Some(err),
            ErrorKind::JsonSerDeser(ref err) => Some(err),
            ErrorKind::JWTCreation(ref err) => Some(err),
            ErrorKind::GenericError(ref _err) => None,
            ErrorKind::InvalidHeaderValueError(ref err) => Some(err),
            ErrorKind::InvalidTestAssertionEvaluation => None,
            ErrorKind::InvalidTestAssertionResponseCheckEvaluation => None,
            ErrorKind::ChannelSendError(ref err) => Some(err),
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

impl From<InvalidHeaderValue> for Error {
    fn from(error: InvalidHeaderValue) -> Error {
        new_error_from(ErrorKind::InvalidHeaderValueError(error))
    }    
}

impl From<SendError<Test>> for Error {
    fn from(error: SendError<Test>) -> Error {
        new_error_from(ErrorKind::ChannelSendError(error))
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