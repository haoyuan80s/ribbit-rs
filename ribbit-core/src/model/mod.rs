// region:   --- Modules

mod embedder;
mod error;
mod store;
mod task;

pub use self::embedder::Embedder;
use self::error::Result;
pub use self::store::{new_db_pool, Db, VecStore};

// endregion: --- Modules

#[derive(Clone)]
pub struct ModelManager<E: Embedder> {
    pub db: Db,
    pub vs: VecStore,
    pub embedder: E,
}

impl<E: Embedder> ModelManager<E> {
    pub async fn from_config() -> Result<Self> {
        let vs = VecStore::from_config().await?;
        Ok(ModelManager {
            db: new_db_pool().await?,
            vs,
            embedder: E::from_config(),
        })
    }
}
