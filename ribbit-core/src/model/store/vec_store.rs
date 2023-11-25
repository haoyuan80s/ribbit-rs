// region:   --- Modules

use qdrant_client::qdrant::point_id::PointIdOptions;
use qdrant_client::qdrant::vectors::VectorsOptions;
use std::collections::HashMap;
use std::sync::Arc;
use std::u64;
use tokio::sync::Mutex;

pub use super::error::{Error, Result};
use crate::config::{config, QdrantCollection};
use qdrant_client::prelude::QdrantClient;
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::{
    CreateCollection, Distance, GetResponse, PointId, PointStruct, SearchPoints, Value, Vector,
    VectorParams, Vectors, VectorsConfig,
};
use tokio::sync::OnceCell;
use tracing::{debug, info};

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
    // pub async fn from_config() -> Result<Self> {
    //     static STORE: OnceCell<VecStore> = OnceCell::const_new();
    //     let vs = STORE
    //         .get_or_init(|| async {
    //             println!("Initializing VecStore");
    //             Self::_from_config().await.unwrap()
    //         })
    //         .await;
    //     // FIXME
    //     // vs.clear_collection(config().qdrant.collections.first().unwrap()) // first collection is for dev
    //     //     .await?;
    //     Ok(vs.clone())
    // }

    pub async fn from_config() -> Result<Self> {
        let qc = QdrantClient::from_url(&config().qdrant.url)
            .build()
            .map_err(|e| {
                Error::QdrantUrlNotFound(format!(
                    "Failed to create qdrant client with error: {}",
                    e
                ))
            })?;
        let vs = VecStore {
            qc: Arc::new(Mutex::new(qc)),
        };

        for clct in &config().qdrant.collections {
            vs.create_collection(clct).await?;
        }
        vs.reset_collection(config().qdrant.collections.first().unwrap()) // first collection is for dev
            .await?;
        Ok(vs)
    }

    pub async fn create_collection(&self, clct: &QdrantCollection) -> Result<()> {
        if self.list_collections().await?.contains(&clct.name) {
            debug!("qd collection {} already exists.", clct.name);
            return Ok(());
        }
        let qc = self.qc.lock().await;
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
        .map_err(|e| Error::QdrantCreateError(format!("Failed to create collection: {}", e)))?;
        Ok(())
    }

    pub async fn list_collections(&self) -> Result<Vec<String>> {
        let qc = self.qc.lock().await;
        let names = qc
            .list_collections()
            .await
            .map_err(|e| Error::QdrantFetchError(e.to_string()))?;
        Ok(names
            .collections
            .into_iter()
            .map(|c| c.name)
            .collect::<Vec<_>>())
    }

    pub async fn delete_collection(&self, name: &str) -> Result<()> {
        let qc = self.qc.lock().await;
        qc.delete_collection(name).await.map_err(|_| {
            Error::QdrantDeleteError(format!("Failed to delete collection: {name}"))
        })?;
        Ok(())
    }

    pub async fn reset_collection(&self, clct: &QdrantCollection) -> Result<()> {
        self.delete_collection(&clct.name).await?;
        self.create_collection(clct).await?;
        Ok(())
    }

    pub async fn update_points(
        &self,
        name: &str,
        id_and_embs: Vec<(i64, Embedding)>,
    ) -> Result<()> {
        let qc = self.qc.lock().await;
        let points = id_and_embs
            .into_iter()
            .map(|p| PointStruct {
                id: Some((p.0 as u64).into()),
                payload: HashMap::default(),
                vectors: Some(p.1.into()),
            })
            .collect();

        qc.upsert_points_blocking(name, points, None)
            .await
            .map_err(|e| Error::QdrantUpdateError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_point_embeddings(&self, name: &str, ids: Vec<u64>) -> Result<Vec<Embedding>> {
        let qc = self.qc.lock().await;
        let ids: Vec<PointId> = ids.into_iter().map(|i| i.into()).collect();
        let point: GetResponse = qc
            .get_points(name, &ids, Some(true), Some(false), None)
            .await
            .map_err(|e| Error::QdrantFetchError(e.to_string()))?;
        Ok(point
            .result
            .into_iter()
            .map(|p| {
                let x = p.vectors.unwrap();
                if let Vectors {
                    vectors_options: Some(VectorsOptions::Vector(vec)),
                } = x
                {
                    vec.data
                } else {
                    panic!("Wrong vector type")
                }
            })
            .collect())
    }

    pub async fn seach_points(
        &self,
        name: &str,
        embedding: Vec<f32>,
        limit: u64,
    ) -> Result<Vec<(i64, f32)>> {
        let qc = self.qc.lock().await;
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
            .filter_map(|p| {
                if let PointId {
                    point_id_options: Some(PointIdOptions::Num(id)),
                } = p.id.unwrap()
                {
                    Some((id as i64, p.score))
                } else {
                    None
                }
            })
            .collect())
    }

    pub async fn delete_points(&self, name: &str, ids: Vec<u64>) -> Result<()> {
        let qc = self.qc.lock().await;
        let ids: Vec<PointId> = ids.into_iter().map(|i| i.into()).collect();
        qc.delete_points(name, &ids.into(), None)
            .await
            .map_err(|e| Error::QdrantDeleteError(format!("Failed to delete points: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use crate::config;

    #[allow(unused)]
    use super::*;
    use anyhow::Result;
    use serial_test::serial;
    use tokio::time::sleep;

    #[serial]
    #[tokio::test]
    async fn test_create_ok() -> Result<()> {
        let vs = VecStore::from_config().await?;
        // let clct = QdrantCollection {
        //     name: "foo".to_string(),
        //     dim: 2,
        //     distance: "Cosine".to_string(),
        // };

        let clct = config().qdrant.collections.first().unwrap();
        vs.create_collection(&clct).await?;
        let names = vs.list_collections().await?;
        assert!(names.contains(&clct.name));
        vs.delete_collection(&clct.name).await?;
        Ok(())
    }

    #[serial]
    #[tokio::test]
    async fn test_delete_ok() -> Result<()> {
        let vs = VecStore::from_config().await?;
        // let clct = QdrantCollection {
        //     name: "foo".to_string(),
        //     dim: 2,
        //     distance: "Cosine".to_string(),
        // };
        let clct = config().qdrant.collections.first().unwrap();
        vs.create_collection(&clct).await?;
        vs.delete_collection(&clct.name).await?;
        let names = vs.list_collections().await?;
        assert!(!names.contains(&clct.name));
        Ok(())
    }

    #[serial]
    #[tokio::test]
    async fn test_save_points_ok() -> Result<()> {
        // let mut vs0 = VecStore::from_config().await?;
        // let x = vs0.list_collections().await?;
        // println!("{:?}", x);
        let mut vs = VecStore::from_config().await?;
        let clct = config().qdrant.collections.first().unwrap();
        vs.create_collection(clct).await?;
        let id_and_embs = vec![
            (1, vec![1.0; clct.dim as usize]),
            (2, vec![2.0; clct.dim as usize]),
        ];
        vs.update_points(&clct.name, id_and_embs).await?;
        let embs = vs.get_point_embeddings(&clct.name, vec![1, 2]).await?;
        assert_eq!(embs.len(), 2);
        vs.delete_collection(&clct.name).await?;
        Ok(())
    }

    #[serial]
    #[tokio::test]
    async fn test_save_points_err_dim_mismatch() -> Result<()> {
        let mut vs = VecStore::from_config().await?;
        let clct = config().qdrant.collections.first().unwrap();
        vs.create_collection(clct).await?;
        let id_and_embs = vec![
            (1, vec![1.0; clct.dim as usize - 1]),
            (2, vec![2.0; clct.dim as usize - 1]),
        ];
        let res = vs.update_points(&clct.name, id_and_embs).await;
        assert!(
            matches!(res, Err(Error::QdrantUpdateError(_))),
            "Expected QdrantUpdateError, got {:?}",
            res
        );
        vs.delete_collection(&clct.name).await?;
        Ok(())
    }

    #[serial]
    #[tokio::test]
    async fn test_search_points_ok() -> Result<()> {
        let mut vs = VecStore::from_config().await?;
        let clct = config().qdrant.collections.first().unwrap();
        vs.create_collection(clct).await?;
        let id_and_embs = vec![
            (1, vec![1.0; clct.dim as usize]),
            (2, vec![2.0; clct.dim as usize]),
        ];
        vs.update_points(&clct.name, id_and_embs).await?;
        let search_result = vs
            .seach_points(&clct.name, vec![1.0; clct.dim as usize], 2)
            .await?;
        assert_eq!(search_result.len(), 2);
        for (id, score) in &search_result {
            let diff = (score - 1.0).abs();
            assert!(diff < 0.0001);
        }
        vs.delete_collection(&clct.name).await?;
        Ok(())
    }
}
// endregion: --- Test
