use super::Error;
use crate::iconpacks::{
    read_sd_iconpack_metadata as read_sd,
    install_sd_iconpack as install_sd,
    list_installed_iconpacks as list_installed,
    uninstall_iconpack as uninstall,
};
use crate::shared::{IconPack, Icon};


use tauri::{AppHandle, Manager, command};

#[command]
pub async fn preview_sd_iconpack(_app: AppHandle, path: &str) -> Result<Option<IconPack>, Error> {
    read_sd(std::path::Path::new(path)).await.map(Some)
        .or_else(|e| {
            log::error!("Failed to read Stream Deck icon pack ({}) metadata: {}", path, e);
            Err(Error { description: "Failed to read Stream Deck icon pack metadata".into() })
        })
}


#[command]
pub async fn install_sd_iconpack(_app: AppHandle, path: &str) -> Result<(), Error> {
    install_sd(std::path::Path::new(path)).await
        .or_else(|e| {
            log::error!("Failed to install Stream Deck icon pack ({}): {}", path, e);
            Err(Error { description: "Failed to install Stream Deck icon pack".into() })
        })
}

#[command]
pub async fn list_installed_iconpacks(_app: AppHandle) -> Result<Vec<IconPack>, Error> {
    list_installed().await
        .or_else(|e| {
            log::error!("Failed to list icon packs: {}", e);
            Err(Error { description: "Failed to list icon packs".into() })
        })
}

#[command]
pub async fn uninstall_iconpack(_app: AppHandle, path: &str) -> Result<(), Error> {
    uninstall(std::path::Path::new(path)).await
        .or_else(|e| {
            log::error!("Failed to uninstall icon pack ({}): {}", path, e);
            Err(Error { description: "Failed to uninstall icon pack".into() })
        })
}