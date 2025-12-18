pub mod manager;

use anyhow::{Error, bail};
use std::{fs, path::Path, io::Read, env};
use zip::ZipArchive;
use crate::shared::{IconPack, Icon, config_dir};
use serde::{Deserialize};
use base64::{engine::general_purpose::STANDARD, Engine as _};


#[derive(Deserialize)]
struct SDIconPackManifest {
    #[serde(rename = "StreamdeckID")]
    id: String,

    #[serde(rename = "Name")]
    name: String,

    #[serde(rename = "Author")]
    author: String,

    #[serde(rename = "Version")]
    version: String,

    #[serde(rename = "Icon")]
    icon: String,
}

static ICON_PACK_FOLDER: &str = "icon_packs";
static SVG_EXT: &str = ".svg";
static PNG_EXT: &str = ".png";
static JPG_EXT: &str = ".jpg";
static JPEG_EXT: &str = ".jpeg";
static SD_ICON_PACK_EXTENSION: &str = "sdIconPack";

fn check_path_is_sd_iconpack(path: &Path) -> Result<bool, Error> {
    if !path.exists() || !path.is_file() {
        bail!("Icon pack path does not exist");
    }

    if !path.extension().is_some_and(|s| s == "streamDeckIconPack") {
        bail!("Icon pack path does not end with .streamDeckIconPack");
    }

    Ok(true)
}

fn find_sd_iconpack_folder_in_archive(archive: &mut ZipArchive<fs::File>) -> Result<String, Error> {
    let ext = format!(".{SD_ICON_PACK_EXTENSION}");
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let name = file.name();
        if name.ends_with('/') && name.trim_end_matches('/').ends_with(&ext) {
            return Ok(name.to_string());
        }
    }

    bail!("No folder ending with {} found in archive", ext);
}

pub async fn read_sd_iconpack_metadata(path: &Path) -> Result<IconPack, Error> {
    // path should lead to a file that ends with .streamDeckIconPack
    check_path_is_sd_iconpack(path)?;

    // path is a zip-like file, we need to read and parse manifest.json from it:
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    // inside of the archive, we expect one of the folders to end with .sdIconPack
    // go through the contents and find the name of that folder:
    let iconpack_folder = find_sd_iconpack_folder_in_archive(&mut archive)?;

    let manifest = {
        let manifest_path = format!("{}manifest.json", iconpack_folder);
        let metadata_file = archive.by_name(&manifest_path)?;
        let manifest: SDIconPackManifest = serde_json::from_reader(metadata_file)?;
        manifest
    };

    // also, read the Icon file and convert it to DataUrl
    let icon_dataurl = {
        let icon_path = format!("{}{}", iconpack_folder, manifest.icon);
        let mut icon_file = archive.by_name(&icon_path)?;
        let mut icon_data = Vec::new();
        icon_file.read_to_end(&mut icon_data)?;


        let icon_base64 = STANDARD.encode(&icon_data);

        // decide icon mime type based on file extension
        let icon_mime = if manifest.icon.ends_with(PNG_EXT) {
            "image/png"
        } else if manifest.icon.ends_with(JPG_EXT) || manifest.icon.ends_with(JPEG_EXT) {
            "image/jpeg"
        } else if manifest.icon.ends_with(SVG_EXT) {
            "image/svg+xml"
        } else {
            "application/octet-stream"
        };
        format!("data:{};base64,{}", icon_mime, icon_base64)
    };

    // create IconPack struct
    let icon_pack = IconPack {
        id: manifest.id,
        name: manifest.name,
        author: manifest.author,
        version: manifest.version,
        icon: Icon::DataUrl { url: icon_dataurl },
        installed_path: None,
    };

    Ok(icon_pack)
}

pub async fn install_sd_iconpack(path: &Path) -> Result<String, Error> {
    check_path_is_sd_iconpack(path)?;

    // ensure the icon packs directory exists
    let icon_packs_dir = config_dir().join(ICON_PACK_FOLDER);
    tokio::fs::create_dir_all(&icon_packs_dir).await?;

    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let iconpack_folder = find_sd_iconpack_folder_in_archive(&mut archive)?;

    // TODO: use tempfile crate to create a temp directory
    // unpack the .streamDeckIconPack file into the temporary directory
    let temp_dir = env::temp_dir();
    archive.extract(&temp_dir)?;

    // move the unpacked pack folder to the icon packs directory
    // this is for the case, when iconpack archive contains more than just the .sdIconPack folder
    let source_path = temp_dir.join(&iconpack_folder);
    let dest_path = icon_packs_dir.join(&iconpack_folder);
    tokio::fs::rename(&source_path, &dest_path).await?;

    Ok(dest_path.display().to_string())
}

pub async fn uninstall_iconpack(path: &Path) -> Result<(), Error> {
    if !path.exists() || !path.is_dir() {
        bail!("Icon pack path does not exist or is not a directory");
    }

    // path must be within the icon packs directory
    let icon_packs_dir = config_dir().join(ICON_PACK_FOLDER);
    if !path.starts_with(&icon_packs_dir) {
        bail!("Icon pack path is not within the icon packs directory");
    }

    tokio::fs::remove_dir_all(path).await?;

    Ok(())
}

pub fn list_installed_iconpacks() -> Result<Vec<IconPack>, Error> {
    let icon_packs_dir = config_dir().join(ICON_PACK_FOLDER);
    let mut icon_packs = Vec::new();

    if !icon_packs_dir.exists() {
        return Ok(icon_packs);
    }

    let mut dir_entries = fs::read_dir(&icon_packs_dir)?;
    while let Some(entry) = dir_entries.next() {
        let path = entry?.path();

        // parsing only those that end with .sdIconPack
        if path.extension().is_some_and(|ext| ext == SD_ICON_PACK_EXTENSION) {
            let get_manifest = || {
                let reader = fs::File::open(&path.join("manifest.json"))?;
                let manifest: SDIconPackManifest = serde_json::from_reader(reader)?;
                let path_str: String = path.to_str().unwrap().to_string();

                Ok::<IconPack, Error>(IconPack {
                    id: manifest.id,
                    name: manifest.name,
                    author: manifest.author,
                    version: manifest.version,
                    icon: Icon::FsPath { path: path.join(manifest.icon).display().to_string() },
                    installed_path: Some(path_str),
                })
            };

            match get_manifest() {
                Ok(pack) => icon_packs.push(pack),
                Err(e) => {
                    log::error!("Failed to read installed Stream Deck icon pack ({}): {}", path.display(), e);
                }
            }
        }
    }

    Ok(icon_packs)
}
