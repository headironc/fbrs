use notify::Error as NotifyError;
use std::sync::mpsc::RecvError as MpscRecvError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("notify error: {0}")]
    Notify(#[from] NotifyError),
    #[error("mpsc recv error: {0}")]
    MpscRecv(#[from] MpscRecvError),
}
