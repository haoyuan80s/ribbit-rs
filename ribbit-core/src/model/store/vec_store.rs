// region:   --- Modules

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub use super::error::{Error, Result};
use crate::config::config;
use qdrant_client::prelude::QdrantClient;
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::{
    CreateCollection, Distance, PointId, PointStruct, SearchPoints, Value, VectorParams,
    VectorsConfig,
};
use tokio::sync::OnceCell;
use tracing::debug;
use uuid::Uuid;

// endregion: --- Modules

#[derive(Clone)]
pub struct VecStore {
    qc: Arc<Mutex<QdrantClient>>,
}

type Embedding = Vec<f32>;

// region:   --- point states

pub trait ScoreState {}
pub struct NoScore;
pub struct WithScore(pub f32);

impl ScoreState for NoScore {}
impl ScoreState for WithScore {}

pub trait EmbeddingState {}
pub struct NoEmbedding;
pub struct WithEmbedding(pub Embedding);
impl EmbeddingState for NoEmbedding {}
impl EmbeddingState for WithEmbedding {}

// pub struct Point<S: ScoreState, E: EmbeddingState> {
//     id: u64,
//     score: S,
//     embedding: E,
// }

// endregion: --- point states

impl VecStore {
    pub async fn from_config() -> Result<Self> {
        static STORE: OnceCell<VecStore> = OnceCell::const_new();
        let x = STORE
            .get_or_init(|| async { Self::_from_config().await.unwrap() })
            .await;
        Ok(x.clone())
    }
    async fn _from_config() -> Result<Self> {
        let qc = QdrantClient::from_url(&config().qdrant.url)
            .build()
            .map_err(|e| {
                Error::QdrantUrlNotFound(format!(
                    "Failed to create qdrant client with error: {}",
                    e
                ))
            })?;
        for clct in &config().qdrant.collections {
            let names = qc.list_collections().await.unwrap();
            if !names.collections.iter().any(|c| c.name == clct.name) {
                qc.create_collection(&CreateCollection {
                    collection_name: clct.name.clone(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: clct.dim,
                            distance: Distance::from_str_name(&clct.distance)
                                .ok_or(Error::InvalidDistanceName(clct.distance.clone()))?
                                .into(),
                            ..Default::default()
                        })),
                    }),
                    ..Default::default()
                })
                .await
                .expect("Failed to create collection");
            }
            debug!("qd collection {} loaded.", clct.name);
        }
        Ok(VecStore {
            qc: Arc::new(Mutex::new(qc)),
        })
    }

    pub async fn delete_collection(&self, name: &str) -> Result<()> {
        let qc = self.qc.lock().unwrap();
        qc.delete_collection(name)
            .await
            .map_err(|e| Error::QdrantDeleteError(format!("Failed to delete collection: {}", e)))?;
        Ok(())
    }

    pub async fn delete_points(&self, name: &str, ids: Vec<u64>) -> Result<()> {
        let qc = self.qc.lock().unwrap();
        let ids: Vec<PointId> = ids.into_iter().map(|i| i.into()).collect();
        qc.delete_points(name, &ids.into(), None)
            .await
            .map_err(|e| Error::QdrantDeleteError(format!("Failed to delete points: {}", e)))?;
        Ok(())
    }

    pub async fn fetch_points(
        &self,
        name: &str,
        embedding: Vec<f32>,
        limit: u64,
    ) -> Result<Vec<(i64, f32)>> {
        let qc = self.qc.lock().unwrap();
        let search_result = qc
            .search_points(&SearchPoints {
                collection_name: name.to_string(),
                vector: embedding,
                limit,
                with_payload: None,
                ..Default::default()
            })
            .await
            .map_err(|e| Error::QdrantFetchError(e.to_string()))?;
        Ok(search_result
            .result
            .into_iter()
            .filter_map(|p| p.payload["id"].as_integer().map(|id| (id, p.score)))
            .collect())
    }

    pub async fn save_points(
        &mut self,
        name: &str,
        id_and_embs: Vec<(i64, Embedding)>,
    ) -> Result<()> {
        let qc = self.qc.lock().unwrap();
        let points = id_and_embs
            .into_iter()
            .map(|p| PointStruct {
                id: Some(Uuid::new_v4().to_string().into()),
                payload: HashMap::from_iter(vec![("id".to_string(), p.0.into())]),
                vectors: Some(p.1.into()),
            })
            .collect();

        qc.upsert_points_blocking(name, points, None)
            .await
            .map_err(|e| Error::QdrantUpdateError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused)]
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_load_ok() -> Result<()> {
        let store = VecStore::from_config().await?;
        Ok(())
    }
    // async fn test_save_ok() -> Result<()> {
    //     let mut store = VecStore::from_config().await?;
    //     let snippet = Snippet {
    //         id: 1,
    //         episode: "episode".to_string(),
    //         show: "show".to_string(),
    //         text: "text".to_string(),
    //         time: None,
    //     };
    //     let embedding = vec![1.0; config().qdrant.EMBEDDING_DIM.try_into().unwrap()];
    //     store.save(snippet, embedding).await?;
    //     Ok(())
    // }
}
// endregion: --- Test
