use http::header::{InvalidHeaderName, InvalidHeaderValue};
use http::method::InvalidMethod;
use notify::Error as NotifyError;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use std::sync::mpsc::RecvError as MpscRecvError;
use tokio::task::JoinError as TokioJoinError;
use url::ParseError as UrlParseError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("notify error: {0}")]
    Notify(#[from] NotifyError),
    // 多个notify error
    #[error("notify errors: {0:?}")]
    Notifies(Vec<NotifyError>),
    #[error("mpsc recv error: {0}")]
    MpscRecv(#[from] MpscRecvError),
    #[error("the specified path is a file: {0}")]
    NotDirectory(String),
    #[error("the specified path does not exist: {0}")]
    DirDoesNotExist(#[from] std::io::Error),
    #[error("tokio join error: {0}")]
    TokioJoin(#[from] TokioJoinError),
    #[error("serde_json error: {0}")]
    SerdeJson(#[from] SerdeJsonError),
    #[error("invalid method: {0}")]
    InvalidMethod(#[from] InvalidMethod),
    #[error("url parse error: {0}")]
    UrlParse(#[from] UrlParseError),
    #[error("invalid header name: {0}")]
    InvalidHeaderName(#[from] InvalidHeaderName),
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] ReqwestError),
    #[error("processor not found: {0}")]
    ProcessorNotFound(String),
}
