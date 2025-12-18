use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use anyhow::Error;

use crate::shared::{IconPack, Icon};
use crate::iconpacks::{
    install_sd_iconpack,
    list_installed_iconpacks,
    uninstall_iconpack,
};

pub struct IconPackManager {
    installed_packs: RwLock<HashMap<String, IconPack>>,
}

impl IconPackManager {
    pub fn new() -> Self {
        Self { installed_packs: RwLock::new(Vec::new()) }
    }

    pub fn refresh(&self) -> Result<(), Error> {
        println!("Refreshing installed icon packs list...");

        let new_packs = list_installed_iconpacks()
            .or_else(|e| {
                log::error!("Failed to list installed icon packs: {}", e);
                Err(e)
            })?;

        {
            let mut installed_packs = self.installed_packs.write().unwrap();
            installed_packs.clear();
            for pack in new_packs {
                installed_packs.insert(pack.id.clone(), pack);
            }
        }

        Ok(())
    }

    pub async fn install_sd_pack_from_file(&self, path: &Path) -> Result<(), Error> {
        install_sd_iconpack(path).await
            .or_else(|e| {
                log::error!("Failed to install Stream Deck icon pack ({}): {}", path.display(), e);
                Err(e)
            })?;

        // rescan installed packs list
        self.refresh()?;

        Ok(())
    }

    pub async fn remove_by_id(&self, id: &str) -> Result<(), Error> {
        let pack_path_opt = {
            let installed_packs = self.installed_packs.read().unwrap();
            installed_packs.get(id)
                .and_then(|pack| pack.installed_path.as_ref().map(|s| s.clone()))
        };

        if let Some(pack_path_str) = pack_path_opt {
            let pack_path = Path::new(&pack_path_str);
            uninstall_iconpack(pack_path).await
                .or_else(|e| {
                    log::error!("Failed to uninstall icon pack ({}): {}", pack_path.display(), e);
                    Err(e)
                })?;

            // rescan installed packs list
            self.refresh()?;
        } else {
            log::warn!("Icon pack with ID '{}' not found among installed packs", id);
        }

        Ok(())
    }

    pub fn get_installed_packs(&self) -> Vec<IconPack> {
        let installed_packs = self.installed_packs.read().unwrap();
        installed_packs.values().cloned().collect()
    }
}
