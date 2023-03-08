use std::fmt::{Display, Formatter};

pub type PeerResult<T> = Result<T, PeerError>;

#[derive(Debug)]
pub struct PeerError {
    pub msg: String,
}

impl Display for PeerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for PeerError {}

impl From<String> for PeerError {
    fn from(msg: String) -> Self {
        PeerError { msg }
    }
}

impl From<&str> for PeerError {
    fn from(msg: &str) -> Self {
        PeerError::from(msg.to_string())
    }
}

impl From<std::io::Error> for PeerError {
    fn from(value: std::io::Error) -> Self {
        Self::from(format!("{}", value))
    }
}
