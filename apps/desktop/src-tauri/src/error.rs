use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("Cannot read or save local data. Check folder permissions and free disk space.")]
    Io(#[from] std::io::Error),
    #[error("The local service request failed. Check that the service is running, then retry.")]
    Http(#[from] reqwest::Error),
    #[error("Local data or the backend response has an unsupported format.")]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
pub fn message(value: impl Into<String>) -> Error {
    Error::Message(value.into())
}
