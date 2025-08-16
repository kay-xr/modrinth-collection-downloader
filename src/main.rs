mod download;
mod modrinth;

use crate::download::download_files;
use crate::modrinth::{
    check_modrinth_status, get_collection_details, get_mod_links, log_project_name,
};
use anyhow::{Context, Result};
use colored::Colorize;
use inquire::validator::Validation;
use inquire::{Select, Text};
use regex::Regex;
use tokio::fs;

pub const MODRINTH_URL: &str = "https://api.modrinth.com/";
pub const MODRINTH_STAGING_URL: &str = "https://staging-api.modrinth.com/";

#[tokio::main]
async fn main() -> Result<()> {
    println!(
        "{}",
        r#"+------------------------------------------------------------------------------------+
|                          Modrinth API Collection Downloader                        |
|                                      Welcome                                       |
|  PLEASE NOTE: This tool makes no assumptions about compatibility.                  |
|  If a mod requires dependencies, it is up to you to add these to your collection   |
|  or download them separately manually.                                             |
|  This tool also assumes every mod will contain the version supplied.               |
|  If a project does not contain a compatible version reported by the API,           |
|  it will be skipped and a message will be shown at the end of the process.         |
+------------------------------------------------------------------------------------+"#
            .bright_green()
    );

    // Get mod platform
    let mod_platform_options = vec!["Fabric", "Neoforge", "Quilt", "Forge"];
    let platform_ans: &str =
        Select::new("What type of server are you running?", mod_platform_options)
            .prompt()
            .map_err(|e| anyhow::anyhow!("Platform selection failed: {e}"))?;

    let mod_platform = match platform_ans {
        "Fabric" => "fabric".to_string(),
        "Neoforge" => "neoforge".to_string(),
        "Quilt" => "quilt".to_string(),
        "Forge" => {
            println!(
                "{}",
                "Warning: It's recommended to use NeoForge in Minecraft 1.20+".bright_red()
            );
            "forge".to_string()
        }
        _ => unreachable!("Unexpected option"),
    };

    // Get version
    let version_validator = |input: &str| {
        // Regex matches release versions like "1.21" and snapshots like "24w31a"
        let re = Regex::new(r"^(1\.\d+(\.\d+)?|[0-9]{2}w[0-9]{2}[a-z])$").unwrap();

        if re.is_match(input) {
            Ok(Validation::Valid)
        } else {
            Ok(Validation::Invalid(
                "Please enter a valid Minecraft version (e.g., 1.21.1 or 24w31a)".into(),
            ))
        }
    };

    let minecraft_version: String = Text::new(
        "Which version of Minecraft are you trying to download for?\nVersion numbers (e.g., 1.21.1) and snapshots (e.g., 24w31a) are accepted."
    )
        .with_default("1.21.1")
        .with_validator(version_validator)
        .prompt()
        .map_err(|e| anyhow::anyhow!("Version prompt failed: {e}"))?;

    // Collection URL prompt (no unwrap)
    let collection_url: String =
        Text::new("What is the URL (or ID) of the collection you are trying to download?")
            .with_default("XXXXXX")
            .prompt()
            .map_err(|e| anyhow::anyhow!("Collection prompt failed: {e}"))?;

    // Check / create directory
    let dir = get_path().await?;

    // Web functions
    check_modrinth_status()
        .await
        .context("Modrinth status check failed: ")?;

    let collection = get_collection_details(collection_url)
        .await
        .context("Getting collection details failed: ")?;
    println!(
        "{}",
        format!("Got {} projects...", collection.projects.len()).bright_green()
    );

    let (links, failed) =
        get_mod_links(collection.projects, mod_platform, minecraft_version).await?;

    download_files(links, dir).await?;

    if failed.len() > 0 {
        let selection_options = vec!["Yes", "No"];
        let platform_ans: &str = Select::new(
            &format!(
                "Failed to get versions for {} files, would you like to see their names?",
                failed.len()
            ),
            selection_options,
        )
        .prompt()
        .map_err(|e| anyhow::anyhow!("Platform selection failed: {e}"))?;

        match platform_ans {
            "Yes" => {
                for failed_file in failed {
                    log_project_name(failed_file).await?;
                }
            }
            _ => {}
        };
    }

    Ok(())
}

pub(crate) async fn get_path() -> Result<String> {
    let mut exe_path = std::env::current_exe()?;
    exe_path.pop();
    exe_path.push("mods");

    if !fs::try_exists(&exe_path).await? {
        fs::create_dir(&exe_path).await?;
    }

    Ok(exe_path.to_str().unwrap().to_string())
}
