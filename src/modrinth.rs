use crate::{MODRINTH_STAGING_URL, MODRINTH_URL};
use anyhow::{Result, anyhow};
use colored::Colorize;
use reqwest::Client;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};

/// Check the status of the Modrinth API
pub async fn check_modrinth_status() -> Result<()> {
    let use_url: &str;

    if cfg!(debug_assertions) {
        println!(
            "{}",
            "Running in debug mode, using staging URL".bright_green()
        );
        use_url = MODRINTH_STAGING_URL;
    } else {
        use_url = MODRINTH_URL;
    }

    let res = reqwest::get(use_url).await?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(anyhow!(format!(
            "An error occured when checking the status of the Modrinth API. {}",
            res.status()
        )))
    }
}

/// Get the details of a collection from modrinth.
pub async fn get_collection_details(url: String) -> Result<Collection> {
    let collection_id = extract_collection_id(url)?;

    let url = format!("https://api.modrinth.com/v3/collection/{}", collection_id);

    let client = Client::new();
    let resp = client
        .get(&url)
        .header(
            USER_AGENT,
            "kay-xr/modrinth_collection_downloader/0.1.0 (archangel@angelware.net)",
        )
        .send()
        .await?;

    if resp.status().is_success() {
        let collection: Collection = resp.json().await?;
        Ok(collection)
    } else {
        Err(anyhow!(format!("Failed with status: {}", resp.status())))
    }
}

/// Get the mod downloads
pub async fn get_mod_links(
    mod_ids: Vec<String>,
    loader: String,
    version: String,
) -> Result<(Vec<ModrinthProject>, Vec<String>)> {
    let mut links: Vec<ModrinthProject> = vec![];
    let mut failed_downloads: Vec<String> = vec![];

    let client = Client::new();
    for mod_id in mod_ids {
        let url = format!(
            "https://api.modrinth.com/v2/project/{}/version?loaders=[\"{}\"]&game_versions=[\"{}\"]",
            mod_id, loader, version
        );

        println!("{}", url.clone());

        let res = client
            .get(&url)
            .header(
                USER_AGENT,
                "kay-xr/modrinth_collection_downloader/0.1.0 (archangel@angelware.net)",
            )
            .send()
            .await?;

        if !res.status().is_success() {
            failed_downloads.push(mod_id);

            println!(
                "Download failed with code {}:\n{}",
                res.status(),
                res.text().await?
            );
            continue;
        } else {
            let json: Vec<ProjectVersion> = res.json().await?;

            // pick the latest
            let latest = json.iter().max_by(|a, b| {
                (a.featured, &a.date_published).cmp(&(b.featured, &b.date_published))
            });

            if let Some(ver) = latest {
                if let Some(file) = ver
                    .files
                    .iter()
                    .find(|f| f.primary)
                    .or_else(|| ver.files.first())
                {
                    let proj: ModrinthProject = ModrinthProject {
                        id: mod_id,
                        name: file.filename.clone(),
                        download_link: file.url.clone(),
                    };

                    links.push(proj);
                    // links.push(file.url.clone());
                } else {
                    failed_downloads.push(mod_id);
                }
            } else {
                failed_downloads.push(mod_id.clone());
            }
        }
    }

    Ok((links, failed_downloads))
}

pub async fn log_project_name(mod_id: String) -> Result<()> {
    let url = format!("https://api.modrinth.com/v2/project/{}", mod_id);

    let client = Client::new();
    let resp = client
        .get(&url)
        .header(
            USER_AGENT,
            "kay-xr/modrinth_collection_downloader/0.1.0 (archangel@angelware.net)",
        )
        .send()
        .await?;

    if resp.status().is_success() {
        let collection: Project = resp.json().await?;

        println!(
            "{}, https://modrinth.com/mod/{}",
            collection.title, collection.slug
        );

        Ok(())
    } else {
        Err(anyhow!(format!("Failed with status: {}", resp.status())))
    }
}

/// Extracts a collection ID from a url, if there is no matching prefix, we just assume it's already
/// an ID.
fn extract_collection_id(input: String) -> Result<String> {
    let prefix = "https://modrinth.com/collection/";

    if input.starts_with(prefix) {
        let id = &input[prefix.len()..];
        if id.is_empty() {
            Err(anyhow!("Collection ID is missing"))
        } else {
            Ok(id.to_string())
        }
    } else if !input.trim().is_empty() {
        Ok(input.to_string())
    } else {
        Err(anyhow!("Invalid collection input"))
    }
}

/// Modrinth collection schema.
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Collection {
    pub id: String,
    pub user: String,
    pub name: String,
    pub description: Option<String>,
    pub projects: Vec<String>,
}

/// Project versions schema.
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct ProjectVersion {
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub name: String,
    pub version_number: String,
    pub featured: bool,
    pub version_type: String,
    pub status: String,
    pub downloads: u64,
    pub changelog: Option<String>,
    pub changelog_url: Option<String>,
    pub date_published: String,
    pub requested_status: Option<String>,

    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<VersionFile>,
    pub dependencies: Vec<Dependency>,
}

/// Files attached to versions.
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub file_type: Option<String>,
    pub hashes: Hashes,
}

/// File version hashes.
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Hashes {
    pub sha1: String,
    pub sha512: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Dependency {
    pub version_id: Option<String>,
    pub project_id: String,
    pub file_name: Option<String>,
    pub dependency_type: String,
}

/// Project names, we only use the title in this project.
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub slug: String,
}

/// Container for Mod details
#[allow(dead_code)]
#[derive(Deserialize, Serialize, Clone)]
pub struct ModrinthProject {
    pub id: String,
    pub name: String,
    pub download_link: String,
}
