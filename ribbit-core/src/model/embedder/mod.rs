mod error;

use crate::config::config;

pub use self::error::{Error, Result};

use async_openai::{
    config::OpenAIConfig,
    types::{CreateEmbeddingRequestArgs, Embedding},
    Client,
};

pub trait Embedder {
    fn from_config() -> Self;
    async fn embeds(&self, text: Vec<&str>) -> Result<Vec<Vec<f32>>>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

pub struct OpenAIEmbedder {
    client: Client<OpenAIConfig>,
    model: String,
}

impl Embedder for OpenAIEmbedder {
    fn from_config() -> Self {
        let client = Client::new();
        let model = config().openai_embedder.model.to_string();
        OpenAIEmbedder { client, model }
    }
    async fn embeds(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(texts)
            .build()
            .map_err(|e| Error::OpenAIEmbedderRequestError(e.to_string()))?;

        let response = self
            .client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| {
                Error::OpenAIEmbedderRequestError(format!("OpenAIEmbedder::embed: {:#?}", e))
            })?;

        let data: Vec<Vec<f32>> = response.data.iter().map(|d| d.embedding.clone()).collect();
        Ok(data)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input([text])
            .build()
            .map_err(|e| Error::OpenAIEmbedderRequestError(e.to_string()))?;

        let response = self
            .client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| {
                Error::OpenAIEmbedderRequestError(format!("OpenAIEmbedder::embed: {:#?}", e))
            })?;

        let data = response.data.first().ok_or_else(|| {
            Error::OpenAIEmbedderRequestError(
                "OpenAIEmbedder::embed: response.data.first() is None".to_string(),
            )
        })?;
        Ok(data.embedding.clone())
    }
}

// region:   --- Test
#[cfg(test)]
mod tests {
    #[allow(unused)]
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_embed_ok() -> Result<()> {
        let embedder = OpenAIEmbedder::from_config();
        let text = "Once upon a time";
        let emb = embedder.embed(text).await?;
        assert_eq!(emb.len(), 1536);
        Ok(())
    }

    #[tokio::test]
    async fn test_embeds_ok() -> Result<()> {
        let embedder = OpenAIEmbedder::from_config();
        let text1 = "Once upon a time";
        let text2 = "Once upon a time again";
        let embs = embedder.embeds(vec![text1, text2]).await?;
        assert_eq!(embs.len(), 2);
        Ok(())
    }
}
// endregion: --- Test
