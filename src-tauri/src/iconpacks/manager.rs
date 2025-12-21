use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use anyhow::Error;

use crate::iconpacks::{
    install_sd_iconpack,
    list_installed_iconpacks,
    list_iconpack_icons,
    uninstall_iconpack,
    index::IconIndex,
    types::{IconPack, PackIcon},
};

pub struct IconPackManager {
    icon_packs_dir: Box<Path>,

    installed_packs: RwLock<HashMap<String, IconPack>>,
    index: IconIndex,
}

impl IconPackManager {
    pub fn new(icon_packs_dir: &Path) -> Self {
        Self {
            icon_packs_dir: icon_packs_dir.into(),
            installed_packs: RwLock::new(HashMap::new()),
            index: IconIndex::new(),
        }
    }

    fn rebuild_index(&self) -> Result<(), Error> {
        self.index.clear();

        let installed_packs = self.installed_packs.read().unwrap();
        for (_id, pack) in installed_packs.iter() {
            let icons = list_iconpack_icons(pack)
                .or_else(|e| {
                    log::error!("Failed to list icons for icon pack ({}): {}", pack.id, e);
                    Err(e)
                })?;

            println!("Adding {} icons from pack '{}' to index...", icons.len(), pack.id);
            icons
                .into_iter()
                .for_each(|icon| {
                    self.index.add_icon(&pack.id, icon);
                });
        }

        println!("Updating icon index...");
        self.index.update_index()?;
        Ok(())
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

        self.rebuild_index()?;

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

    pub fn search_icons(&self, query: &str) -> Result<Vec<PackIcon>, Error> {
        self.index.search(query)
    }

    pub fn get_icon_path(&self, pack_id: &str, icon_name: &str) -> Option<PathBuf> {
        let installed_packs = self.installed_packs.read().unwrap();
        let pack = installed_packs.get(pack_id)?;

        let path = {
            let pack_icons_path = pack.installed_path.as_ref().map(|s| Path::new(s).join("icons"))?;

            self.index
                .get_icon(&pack.id, icon_name)
                .map(|icon| pack_icons_path.join(&icon.icon.file_name))
                .take_if(|p| p.is_file())
        };

        path
    }
}
