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
    const DB_TABLE_NAME: &'static str;
}

pub struct TaskBmc;

impl VsBmc for TaskBmc {
    const COLLECTION_NAME: &'static str = "task";
    const DB_TABLE_NAME: &'static str = "story";
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Task {
    pub id: i64,
    pub story: String, // Keep simple first
}

#[derive(Deserialize, Clone)]
pub struct TaskForCreate {
    pub story: String,
}

#[derive(Deserialize, Clone)]
pub struct TaskForUpdate {
    pub id: i64,
    pub story: String,
}

// endregion: --- Task Types

impl TaskBmc {
    pub async fn create(
        _ctx: Ctx,
        mm: ModelManager<impl Embedder>,
        task: TaskForCreate,
    ) -> Result<i64> {
        let (id,): (i64,) = sqlx::query_as(
            "
            INSERT INTO story (story) VALUES ($1) RETURNING id
            ",
        )
        .bind(&task.story)
        .fetch_one(&mm.db)
        .await?;

        let emb = mm.embedder.embed(&task.story).await?;
        mm.vs
            .update_points(Self::COLLECTION_NAME, vec![(id, emb)])
            .await?;

        Ok(id)
    }

    pub async fn update(
        _ctx: Ctx,
        mm: ModelManager<impl Embedder>,
        task: TaskForUpdate,
    ) -> Result<()> {
        let count = sqlx::query(
            "
            UPDATE story SET story = $1 WHERE id = $2
            ",
        )
        .bind(&task.story)
        .bind(task.id)
        .execute(&mm.db)
        .await?
        .rows_affected();
        if count == 0 {
            return Err(Error::EntityNotFound {
                entity: Self::COLLECTION_NAME,
                id: task.id,
            });
        };
        let emb = mm.embedder.embed(&task.story).await?;
        mm.vs
            .update_points(Self::COLLECTION_NAME, vec![(task.id, emb)])
            .await?;

        Ok(())
    }

    pub async fn read(_ctx: Ctx, mm: ModelManager<impl Embedder>, id: i64) -> Result<Task> {
        let task = sqlx::query_as(
            "
            SELECT id, story FROM story WHERE id = $1
            ",
        )
        .bind(id)
        .fetch_one(&mm.db)
        .await?;
        Ok(task)
    }

    pub async fn delete(_ctx: Ctx, mm: ModelManager<impl Embedder>, id: i64) -> Result<()> {
        let count = sqlx::query(
            "
            DELETE FROM story WHERE id = $1
            ",
        )
        .bind(id)
        .execute(&mm.db)
        .await?
        .rows_affected();
        if count == 0 {
            return Err(Error::EntityNotFound {
                entity: Self::COLLECTION_NAME,
                id,
            });
        };
        mm.vs
            .delete_points(Self::COLLECTION_NAME, vec![id as u64])
            .await?;
        Ok(())
    }
}

// TODO testing
// region:   --- Test
#[cfg(test)]
mod tests {
    use crate::{_dev_utils, model::embedder::OpenAIEmbedder};

    #[allow(unused)]
    use super::*;
    use anyhow::Result;
    use serial_test::serial;

    #[serial]
    #[tokio::test]
    async fn test_create_ok() -> Result<()> {
        _dev_utils::init_dev().await;
        let ctx = Ctx::root_ctx();
        let mm = ModelManager::<OpenAIEmbedder>::from_config().await?;
        let task = TaskForCreate {
            story: "This is a story".to_string(),
        };
        let id = TaskBmc::create(ctx.clone(), mm.clone(), task).await?;
        let embs = mm
            .vs
            .get_point_embeddings(TaskBmc::COLLECTION_NAME, vec![id as u64])
            .await?;
        assert_eq!(embs.len(), 1);
        TaskBmc::delete(ctx, mm, id).await?;
        Ok(())
    }

    #[serial]
    #[tokio::test]
    async fn test_read_ok() -> Result<()> {
        _dev_utils::init_dev().await;
        let ctx = Ctx::root_ctx();
        let mm = ModelManager::<OpenAIEmbedder>::from_config().await?;
        let task = TaskForCreate {
            story: "This is a story".to_string(),
        };
        let id = TaskBmc::create(ctx.clone(), mm.clone(), task).await?;
        let task = TaskBmc::read(ctx.clone(), mm.clone(), id).await?;
        assert_eq!(task.story, "This is a story");

        TaskBmc::delete(ctx, mm, id).await?;
        Ok(())
    }

    #[serial]
    #[tokio::test]
    async fn test_update_ok() -> Result<()> {
        _dev_utils::init_dev().await;
        let ctx = Ctx::root_ctx();
        let mm = ModelManager::<OpenAIEmbedder>::from_config().await?;
        let task_for_create = TaskForCreate {
            story: "This is a story".to_string(),
        };
        let id = TaskBmc::create(ctx.clone(), mm.clone(), task_for_create.clone()).await?;
        let task_for_update = TaskForUpdate {
            id,
            story: "This is a new story".to_string(),
        };
        TaskBmc::update(ctx.clone(), mm.clone(), task_for_update).await?;
        let task = TaskBmc::read(ctx.clone(), mm.clone(), id).await?;
        assert_eq!(task.story, "This is a new story");
        TaskBmc::delete(ctx, mm, id).await?;
        Ok(())
    }

    #[serial]
    #[tokio::test]
    async fn test_delete_ok() -> Result<()> {
        _dev_utils::init_dev().await;
        let ctx = Ctx::root_ctx();
        let mm = ModelManager::<OpenAIEmbedder>::from_config().await?;
        let task = TaskForCreate {
            story: "This is a story".to_string(),
        };
        let id = TaskBmc::create(ctx.clone(), mm.clone(), task).await?;
        TaskBmc::delete(ctx.clone(), mm.clone(), id).await?;
        let task = TaskBmc::read(ctx.clone(), mm.clone(), id).await;
        assert!(task.is_err());
        Ok(())
    }
}
// endregion: --- Test
