use crate::error::{Error, Result};
use dotenvy::dotenv;
use serde::Deserialize;
use std::{env, fs, sync::OnceLock};

pub fn config() -> &'static Config {
    static INSTANCE: OnceLock<Config> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        Config::load_from_env()
            .unwrap_or_else(|er| panic!("Failed to load config with error: {er}"))
    })
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: Database,
    pub qdrant: Qdrant,
    pub openai_embedder: OpenAIEmbedder,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub db_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Qdrant {
    pub url: String,
    pub collections: Vec<QdrantCollection>,
}

#[derive(Debug, Deserialize)]
pub struct QdrantCollection {
    pub name: String,
    pub dim: u64,
    pub distance: String,
}
#[derive(Debug, Deserialize)]
pub struct OpenAIEmbedder {
    pub model: String,
}

impl Config {
    pub fn load_from_env() -> Result<Config> {
        dotenv().map_err(|_| Error::DotEnvNotFound)?;
        let conf_path = env::var("CONF_PATH").map_err(|_| Error::ConfigMissingEnv("CONF_PATH"))?;
        let conf_str = fs::read_to_string(conf_path.clone())
            .map_err(|_| Error::ConfigReadConfigFile(conf_path.clone()))?;
        let conf: Config =
            serde_json::from_str(&conf_str).map_err(|_| Error::ConfigParseConfigFile(conf_path))?;
        Ok(conf)
    }
}

// region:   --- Test
#[cfg(test)]
mod tests {
    #[allow(unused)]
    use super::*;
    use anyhow::Result;
    #[test]
    fn test_load_from_env_ok() -> Result<()> {
        let conf = Config::load_from_env()?;
        println!("{:?}", conf);
        Ok(())
    }
}
// endregion: --- Test
