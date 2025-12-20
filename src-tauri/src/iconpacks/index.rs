use crate::iconpacks::{
	interner::{StringId, StringInterner},
	types::{Icon, PackIcon},
};
use anyhow::{Error, bail};
use fst::{IntoStreamer, Map, MapBuilder, Streamer};
use regex_automata::dense;
use std::{collections::HashMap, sync::RwLock};
use std::io::Cursor;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct IconId {
    pack: StringId,
    name: StringId,
}

impl IconId {
    pub fn new(pack: &str, name: &str, interner: &StringInterner) -> Self {
        Self {
            pack: interner.intern(pack),
            name: interner.intern(name),
        }
    }

    pub fn to_u64(&self) -> u64 {
        ((self.pack as u64) << 32) | (self.name as u64)
    }

    pub fn from_u64(value: u64) -> Self {
        let pack = (value >> 32) as u32;
        let name = (value & 0xFFFFFFFF) as u32;
        Self { pack, name }
    }
}

type BuildIndex = Vec<u8>;

pub struct IconIndex {
    interner: StringInterner,
	icons: RwLock<HashMap<IconId, PackIcon>>,
	built_index: RwLock<BuildIndex>,
}

impl IconIndex {
	pub fn new() -> Self {
		Self {
            interner: StringInterner::new(),
			icons: RwLock::new(HashMap::new()),
			built_index: RwLock::new(Vec::new()),
		}
	}

	pub fn add_icon(&self, pack: &str, icon: Icon) {
		let icon_id = IconId::new(pack, &icon.name, &self.interner);

        self.icons
            .write().unwrap()
            .insert(icon_id, PackIcon { pack_id: pack.to_string(), icon });
	}

    pub fn clear(&self) {
        // recreate StringInterner

        let mut icons = self.icons.write().unwrap();
        icons.clear();

        let mut map_lock = self.built_index.write().unwrap();
        map_lock.clear();

        self.interner.clear();
    }

	fn build_search_index(&self) -> Result<BuildIndex, Error> {
        let icons = self.icons.read().unwrap();

        let sorted_icon_entries = {
            // iterate over icons, form a search key by concatenating pack name and icon name, sort the array by key
            let mut icon_entries: Vec<(String, u64)> = icons
                .iter()
                .map(|(icon_id, pack_icon)| {
                    let key = format!("pack:{}|name:{}", pack_icon.pack_id, pack_icon.icon.name);
                    (key, icon_id.to_u64())
                })
                .collect();

            icon_entries.sort_by(|a, b| a.0.cmp(&b.0));
            icon_entries
        };

        let fst_buffer = {
            let mut fst_buffer = Vec::new();
            let cursor = Cursor::new(&mut fst_buffer);
            let mut builder = MapBuilder::new(cursor)?;
            for (key, id) in sorted_icon_entries.iter() {
                builder.insert(key, *id)?;
            }

            builder.finish()?;
            fst_buffer
        };

        println!("Built icon search index of {} icons, size: {}b", icons.len(), fst_buffer.len());

        Ok(fst_buffer)
	}

    pub fn update_index(&self) -> Result<(), Error> {
        let map = self.build_search_index()?;
        let mut map_lock = self.built_index.write().unwrap();
        *map_lock = map;

        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<Vec<PackIcon>, Error> {
        let map_lock = self.built_index.read().unwrap();

        let map = {
            let buf = &*map_lock;
            if buf.is_empty() {
                bail!("Icon index is empty");
            }

            println!("Searching for icons matching query: {} in index that is {}b big", query, buf.len());
            Map::new(buf)?
        };

        let results = {
            let icons = self.icons.read().unwrap();
            let mut results = vec![];

            let matcher = dense::Builder::new()
                .build(&query)
                .unwrap();

            let mut stream = map.search(&matcher).into_stream();
            while let Some((_, icon_id_u64)) = stream.next() {
                let icon_id = IconId::from_u64(icon_id_u64);
                if let Some(icon) = icons.get(&icon_id) {
                    results.push(icon.to_owned());
                }
            }

            results
        };

        Ok(results)
    }
}
