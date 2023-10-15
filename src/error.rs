use notify::Error as NotifyError;
use std::sync::mpsc::RecvError as MpscRecvError;
use tokio::task::JoinError as TokioJoinError;

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
}
