use super::Error;
use tauri::{command, State};
use crate::iconpacks::{
    manager::IconPackManager, read_sd_iconpack_metadata as read_sd, types::{Icon, IconPack}
};

use serde::{Serialize, Deserialize};

#[command]
pub async fn preview_sd_iconpack(path: &str) -> Result<Option<IconPack>, Error> {
    read_sd(std::path::Path::new(path)).await.map(Some)
        .or_else(|e| {
            log::error!("Failed to read Stream Deck icon pack ({}) metadata: {}", path, e);
            Err(Error { description: "Failed to read Stream Deck icon pack metadata".into() })
        })
}


#[command]
pub async fn install_sd_iconpack(path: &str, manager: State<'_, IconPackManager>) -> Result<(), Error> {
    match manager.install_sd_pack_from_file(std::path::Path::new(path)).await {
        Ok(_) => Ok(()),
        Err(_) => {
            Err(Error { description: "Failed to install Stream Deck icon pack".into() })
        }
    }
}

#[command]
pub async fn list_installed_iconpacks(manager: State<'_, IconPackManager>) -> Result<Vec<IconPack>, Error> {
    Ok(manager.get_installed_packs())
}

#[command]
pub async fn uninstall_iconpack(id: &str, manager: State<'_, IconPackManager>) -> Result<(), Error> {
    manager.remove_by_id(id).await
        .or_else(|e| {
            log::error!("Failed to uninstall icon pack ({}): {}", id, e);
            Err(Error { description: "Failed to uninstall icon pack".into() })
        })
}

#[derive(Serialize, Deserialize)]
pub struct IconSearchResult {
    pub pack: String,
    pub name: String,
}

#[command]
pub async fn search_icons(query: &str, manager: State<'_, IconPackManager>) -> Result<Vec<IconSearchResult>, Error> {
    manager.search_icons(query)?.into_iter()
        .map(|icon| Ok(IconSearchResult {
            pack: icon.pack_id,
            name: icon.icon.name,
        }))
        .collect()
}

#[command]
pub async fn get_icon_path(icon: IconSearchResult, manager: State<'_, IconPackManager>) -> Result<String, Error> {
    manager.get_icon_path(&icon.pack, &icon.name)
        .map(|path| path.display().to_string())
        .ok_or_else(|| Error { description: "Icon path not found".into() })
}
