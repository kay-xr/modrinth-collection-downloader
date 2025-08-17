// Basic packwiz creation, gets you started at least.

use crate::modrinth::ModrinthProject;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::BufReader;
use tokio::process::Command;

#[cfg(target_os = "windows")]
const BIN_NAME: &str = "packwiz.exe";
#[cfg(not(target_os = "windows"))]
const BIN_NAME: &str = "packwiz";

#[cfg(target_os = "windows")]
const ZIP_URL: &str = "https://nightly.link/packwiz/packwiz/workflows/go/main/Windows%2064-bit.zip";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const ZIP_URL: &str =
    "https://nightly.link/packwiz/packwiz/workflows/go/main/Linux%2064-bit%20x86.zip";

pub async fn init_packwiz(mut dir: PathBuf) -> Result<()> {
    dir.pop();
    let packwiz_dir = dir.join("packwiz");
    fs::create_dir_all(&packwiz_dir).await?;

    if command_works("packwiz").await {
        return Ok(());
    }

    // If we don't have packwiz installed we can download the binary, thanks to their cool static url :)
    let bytes = reqwest::get(ZIP_URL)
        .await
        .with_context(|| format!("GET {}", ZIP_URL))?
        .bytes()
        .await
        .context("reading zip payload")?;

    let tmp_zip = packwiz_dir.join("packwiz.zip");
    fs::write(&tmp_zip, &bytes).await?;

    use async_zip::tokio::read::seek::ZipFileReader;
    let file = fs::File::open(&tmp_zip).await?;
    let mut buf = BufReader::new(file);
    let mut reader = ZipFileReader::with_tokio(&mut buf)
        .await
        .context("opening zip")?;

    // find binary
    let entries = reader.file().entries();
    let mut target_idx = None;
    for (i, e) in entries.iter().enumerate() {
        if let Ok(name) = e.filename().as_str() {
            if name.ends_with(BIN_NAME) {
                target_idx = Some(i);
                break;
            }
        }
    }
    let idx = target_idx.context("packwiz binary not found in zip")?;

    // read the entry into memory
    let mut bin_bytes = Vec::new();
    {
        let mut entry_reader = reader.reader_with_entry(idx).await?;
        entry_reader
            .read_to_end_checked(&mut bin_bytes)
            .await
            .context("reading packwiz entry")?;
    }

    // write out binary
    let out_path = packwiz_dir.join(BIN_NAME);
    fs::write(&out_path, &bin_bytes).await?;

    // chmod on unix
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&out_path).await?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&out_path, perms).await?;
    }

    // verify it runs
    let ok = Command::new(&out_path).arg("-h").status().await?.success();
    anyhow::ensure!(ok, "downloaded packwiz failed to run");

    // cleanup zip
    let _ = fs::remove_file(&tmp_zip).await;

    Ok(())
}

pub async fn create_pack(mut dir: PathBuf, list: Vec<ModrinthProject>) -> Result<()> {
    dir.pop();
    let exe = dir.join("packwiz").join(BIN_NAME);
    let run_path = dir.join("packwiz");
    let toml_path = dir.join("packwiz").join("pack.toml");

    // Check if the pack is already initialized
    if !fs::try_exists(toml_path).await? {
        let status = Command::new(&exe)
            .arg("init")
            .current_dir(&run_path)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .context("running packwiz init")?;
        anyhow::ensure!(status.success(), "packwiz init failed");
    } else {
        println!("A pack.toml file already exists, skipping initialization.");
    }

    // Add mods to the pack
    for project in list {
        let status = Command::new(&exe)
            .arg("mr")
            .arg("install")
            .arg(project.id)
            .current_dir(&run_path)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .context("adding mod to pack")?;
        anyhow::ensure!(status.success(), "adding mod(s) failed");
    }

    // Create the mrpack
    let status = Command::new(&exe)
        .arg("mr")
        .arg("export")
        .current_dir(&run_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .context("exporting modpack")?;
    anyhow::ensure!(status.success(), "exporting modpack failed");

    Ok(())
}

async fn command_works(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("-h")
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}
