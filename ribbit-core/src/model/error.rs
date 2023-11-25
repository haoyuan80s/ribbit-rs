use super::{embedder, store};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Store(store::Error),
    Embedder(embedder::Error),
    Sqlx(sqlx::Error),
}

// region:    --- Error Boilerplate
impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}
impl std::error::Error for Error {}
// endregion: --- Error Boilerplate

// region:   --- error from
impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Self::Sqlx(err)
    }
}

impl From<store::Error> for Error {
    fn from(err: store::Error) -> Self {
        Self::Store(err)
    }
}

impl From<embedder::Error> for Error {
    fn from(err: embedder::Error) -> Self {
        Self::Embedder(err)
    }
}

// endregion: --- error from
