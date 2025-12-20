use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IconType {
	#[serde(rename = "fs")]
	FsPath { path: String },

	#[serde(rename = "dataurl")]
	DataUrl { url: String },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct IconPack {
	pub id: String,
	pub name: String,
	pub author: String,
	pub version: String,
	pub icon: IconType,
	pub installed_path: Option<String>,
}


#[derive(Clone)]
pub struct Icon {
    pub name: String,
    pub file_name: String,
}

impl Icon {
    pub fn new(name: &str, file_name: &str) -> Self {
        Self {
            name: name.to_string(),
            file_name: file_name.to_string(),
        }
    }
}


#[derive(Clone)]
pub struct PackIcon {
    pub pack_id: String,
    pub icon: Icon,
}
