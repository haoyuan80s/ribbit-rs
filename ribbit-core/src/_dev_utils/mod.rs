use tokio::sync::OnceCell;
use tracing::info;

use crate::{model::VecStore, config};

mod dev_db;

pub async fn init_dev() {
    static INIT: OnceCell<()> = OnceCell::const_new();

    INIT.get_or_init(|| async {
        info!("{:<12} - init_dev_all()", "FOR-DEV-ONLY");

        dev_db::init_dev_db().await.unwrap();
    })
    .await;

    let _ = VecStore::from_config()
        .await
        .unwrap()
        .reset_collection(config().qdrant.collections.first().unwrap())
        // .delete_collection(VS_COLLECTION_NAME)
        .await;
}

pub async fn seed_tasks() {
    todo!()
}
