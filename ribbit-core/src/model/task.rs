use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::ctx::Ctx;
use crate::model::error::{Error, Result};
use crate::model::ModelManager;

use super::embedder::Embedder;

///
/// Create: given a long story, truncate it to a short snippets and save to db
///     - need a db -> qdrant transform

pub trait VsBmc {
    const COLLECTION_NAME: &'static str;
}

pub struct TaskBmc;

impl VsBmc for TaskBmc {
    const COLLECTION_NAME: &'static str = "task";
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Task {
    pub id: i64,
    pub story: String, // Keep simple first
}

#[derive(Deserialize)]
pub struct TaskForCreate {
    pub story: String,
}

#[derive(Deserialize)]
pub struct TaskForUpdate {
    pub story: String,
}

// endregion: --- Task Types

impl TaskBmc {
    pub async fn create(
        ctx: Ctx,
        mm: ModelManager<impl Embedder>,
        task: TaskForCreate,
    ) -> Result<()> {
        let mut vs = mm.vs.lock().unwrap();
        let (id,): (i64,) = sqlx::query_as(
            "
            INSERT INTO story (story) VALUES ($1) RETURNING id
            ",
        )
        .bind(&task.story)
        .fetch_one(&mm.db)
        .await?;

        let emb = mm.embedder.embed(&task.story).await?;
        vs.save_points(Self::COLLECTION_NAME, vec![(id, emb)])
            .await?;

        Ok(())
    }
}

// TODO testing
// region:   --- Test
#[cfg(test)]
mod tests {
    use crate::{model::embedder::OpenAIEmbedder, _dev_utils};

    #[allow(unused)]
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_create_ok() -> Result<()> {
        _dev_utils::init_dev().await;
        let ctx = Ctx::root_ctx();
        let mm = ModelManager::<OpenAIEmbedder>::from_config().await?;
        let task = TaskForCreate {
            story: "story".to_string(),
        };
        TaskBmc::create(ctx, mm, task).await?;
        Ok(())
    }
}
// endregion: --- Test
