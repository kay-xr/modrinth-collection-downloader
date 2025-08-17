use crate::modrinth::ModrinthProject;
use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;
use tokio::fs;

/// Creates a log of all mod projects & ids in a json file. Useful for packwiz, etc.
pub async fn create_log_file(links: Vec<ModrinthProject>, failed: Vec<String>, path: PathBuf) -> Result<()> {
    let file_path = path.join("collection.json");

    let collection_log: ModrinthLog = ModrinthLog {
        ids: links.iter().map(|proj| proj.id.clone()).collect(),
        projects: links,
        failed_ids: failed
    };

    let toml_str = serde_json::to_string_pretty(&collection_log)?;
    fs::write(file_path, toml_str).await?;

    Ok(())
}

#[derive(Serialize)]
pub struct ModrinthLog {
    pub ids: Vec<String>,
    pub projects: Vec<ModrinthProject>,
    pub failed_ids: Vec<String>,
}
