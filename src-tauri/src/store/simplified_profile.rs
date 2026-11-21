//! Duplicates of many structs to facilitate saving profiles to disk in a format that can be transferred between devices or systems.

use crate::shared::{Action, ActionContext, ActionInstance, ActionState, Profile, ProfilePage};

use std::{
	fs,
	path::{Path, PathBuf},
};

use path_slash::{PathBufExt, PathExt};
use serde::{Deserialize, Serialize};

#[derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr)]
pub struct DiskActionContext {
	pub page: u8,
	pub controller: String,
	pub position: u8,
	pub index: u16,
}

impl std::fmt::Display for DiskActionContext {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if self.page == 0 {
			write!(f, "{}.{}.{}", self.controller, self.position, self.index)
		} else {
			write!(f, "{}.{}.{}.{}", self.page, self.controller, self.position, self.index)
		}
	}
}

impl std::str::FromStr for DiskActionContext {
	type Err = std::num::ParseIntError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let segments: Vec<&str> = s.split('.').collect();
		match segments.len() {
			6 => Ok(Self {
				page: u8::from_str(segments[2])?,
				controller: segments[3].to_owned(),
				position: u8::from_str(segments[4])?,
				index: u16::from_str(segments[5])?,
			}),
			5 => Ok(Self {
				page: 0,
				controller: segments[2].to_owned(),
				position: u8::from_str(segments[3])?,
				index: u16::from_str(segments[4])?,
			}),
			4 => Ok(Self {
				page: u8::from_str(segments[0])?,
				controller: segments[1].to_owned(),
				position: u8::from_str(segments[2])?,
				index: u16::from_str(segments[3])?,
			}),
			3 => Ok(Self {
				page: 0,
				controller: segments[0].to_owned(),
				position: u8::from_str(segments[1])?,
				index: u16::from_str(segments[2])?,
			}),
			_ => Err("".parse::<u8>().unwrap_err()),
		}
	}
}

impl From<ActionContext> for DiskActionContext {
	fn from(value: ActionContext) -> Self {
		Self {
			page: value.page,
			controller: value.controller,
			position: value.position,
			index: value.index,
		}
	}
}

impl DiskActionContext {
	fn into_action_context(self, device: String, profile: String) -> ActionContext {
		ActionContext {
			device,
			profile,
			page: self.page,
			controller: self.controller,
			position: self.position,
			index: self.index,
		}
	}
}

#[derive(Serialize, Deserialize)]
pub struct DiskActionInstance {
	pub action: Action,
	pub context: DiskActionContext,
	pub states: Vec<ActionState>,
	pub current_state: u16,
	pub settings: serde_json::Value,
	pub children: Option<Vec<DiskActionInstance>>,
}

impl From<ActionInstance> for DiskActionInstance {
	fn from(mut value: ActionInstance) -> Self {
		let disk_context: DiskActionContext = value.context.clone().into();
		let config_dir = crate::shared::config_dir();
		let image_dir = config_dir.join("images").join(&value.context.device).join(&value.context.profile).join(disk_context.to_string());

		let normalise_path = |value: &str| -> String {
			let path = Path::new(value);
			if path.starts_with(&image_dir) {
				path.strip_prefix(&image_dir).unwrap().to_slash_lossy().into_owned()
			} else if path.starts_with(&config_dir) {
				path.strip_prefix(&config_dir).unwrap().to_slash_lossy().into_owned()
			} else {
				path.to_slash_lossy().into_owned()
			}
		};

		let normalise_image = |index: usize, image: &mut String, suffix: &str| {
			if image.trim() == "data:" {
				*image = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIW2NgYGD4DwABBAEAwS2OUAAAAABJRU5ErkJggg==".to_owned();
			}

			if image.starts_with("data:") && image.contains('/') && image.contains(',') {
				let mut extension = image.split_once('/').unwrap().1.split_once(',').unwrap().0;
				if extension.contains(';') {
					extension = extension.split_once(';').unwrap().0;
				}
				if extension.contains('+') {
					extension = extension.split_once('+').unwrap().0;
				}

				let data = if image.contains(";base64,") {
					use base64::Engine;
					let Ok(data) = base64::engine::general_purpose::STANDARD.decode(image.split_once(";base64,").unwrap().1) else {
						return;
					};
					data
				} else {
					image.split_once(',').unwrap().1.as_bytes().to_vec()
				};

				let filename = format!("{}{}.{}", index, suffix, extension);
				// Same blank placeholder as set_image. Do not overwrite the saved button art with it.
				if data.len() < 256 {
					if image_dir.join(&filename).is_file() {
						*image = filename;
					} else {
						return;
					}
				} else if fs::create_dir_all(&image_dir).is_err() || fs::write(image_dir.join(&filename), data).is_err() {
					return;
				} else {
					*image = filename;
				}
			}

			*image = normalise_path(image);
		};

		for (index, state) in value.states.iter_mut().enumerate() {
			if state.text_override.as_ref().map(|s| s.is_empty()).unwrap_or(false) {
				state.text_override = None;
			}

			if let Some(ref mut image) = state.image_override {
				normalise_image(index, image, "-user");
			}
			normalise_image(index, &mut state.image, "");
		}
		for state in value.action.states.iter_mut() {
			state.image = normalise_path(&state.image);
		}
		value.action.icon = normalise_path(&value.action.icon);
		value.action.property_inspector = normalise_path(&value.action.property_inspector);

		Self {
			context: disk_context,
			action: value.action,
			states: value.states,
			current_state: value.current_state,
			settings: value.settings,
			children: value.children.map(|c| c.into_iter().map(|v| v.into()).collect()),
		}
	}
}

impl DiskActionInstance {
	fn into_action_instance(self, path: &Path) -> ActionInstance {
		let config_dir = crate::shared::config_dir();
		let mut iter = path.strip_prefix(&config_dir).unwrap().iter();
		let device = iter.nth(1).unwrap().to_string_lossy().into_owned();
		let mut profile = iter.map(|x| x.to_string_lossy()).collect::<Vec<_>>().join("/");
		profile = profile[..profile.len() - 5].to_owned();

		let reconstruct_path = |value: &str| -> String {
			if !(value.is_empty() || value.starts_with("data:") || value.starts_with("opendeck/")) {
				config_dir.join(PathBuf::from_slash(value)).to_string_lossy().into_owned()
			} else {
				value.to_owned()
			}
		};

		let reconstruct_image = |image: &mut String| {
			if let Some(true) = image.chars().next().map(|v| v.is_numeric()) {
				*image = config_dir
					.join("images")
					.join(&device)
					.join(&profile)
					.join(self.context.to_string())
					.join(&image)
					.to_string_lossy()
					.into_owned();
			} else {
				*image = reconstruct_path(image);
			}
		};

		let mut states = self.states.clone();
		for state in states.iter_mut() {
			if let Some(ref mut image) = state.image_override {
				reconstruct_image(image);
			}
			reconstruct_image(&mut state.image);
		}
		let mut action = self.action.clone();
		for state in action.states.iter_mut() {
			state.image = reconstruct_path(&state.image);
		}
		action.icon = reconstruct_path(&action.icon);
		action.property_inspector = reconstruct_path(&action.property_inspector);

		ActionInstance {
			context: self.context.into_action_context(device, profile),
			action,
			states,
			current_state: self.current_state,
			settings: self.settings,
			children: self.children.map(|c| c.into_iter().map(|v| v.into_action_instance(path)).collect()),
		}
	}
}

#[derive(Serialize, Deserialize)]
pub struct DiskProfilePage {
	#[serde(default)]
	pub keys: Vec<Option<DiskActionInstance>>,
	#[serde(default)]
	pub sliders: Vec<Option<DiskActionInstance>>,
	#[serde(default)]
	pub infobars: Vec<Option<DiskActionInstance>>,
}

#[derive(Serialize, Deserialize)]
pub struct DiskProfile {
	#[serde(default)]
	pub pages: Vec<DiskProfilePage>,
	#[serde(default)]
	pub current_page: usize,
	pub keys: Vec<Option<DiskActionInstance>>,
	pub sliders: Vec<Option<DiskActionInstance>>,
	#[serde(default)]
	pub infobars: Vec<Option<DiskActionInstance>>,
}

fn to_disk_page(page: &ProfilePage) -> DiskProfilePage {
	DiskProfilePage {
		keys: page.keys.clone().into_iter().map(|x| x.map(|v| v.into())).collect(),
		sliders: page.sliders.clone().into_iter().map(|x| x.map(|v| v.into())).collect(),
		infobars: page.infobars.clone().into_iter().map(|x| x.map(|v| v.into())).collect(),
	}
}

impl From<&Profile> for DiskProfile {
	fn from(value: &Profile) -> Self {
		let mut profile = value.clone();
		profile.ensure_pages();
		let current = to_disk_page(profile.current());
		Self {
			pages: profile.pages.iter().map(to_disk_page).collect(),
			current_page: profile.current_page_index(),
			keys: current.keys,
			sliders: current.sliders,
			infobars: current.infobars,
		}
	}
}

impl DiskProfile {
	fn into_profile(self, path: &Path) -> Profile {
		let config_dir = crate::shared::config_dir();
		let mut iter = path.strip_prefix(config_dir).unwrap().iter();
		let _ = iter.nth(1);
		let mut id = iter.map(|x| x.to_string_lossy()).collect::<Vec<_>>().join("/");
		id = id[..id.len() - 5].to_owned();

		let map_slots = |slots: Vec<Option<DiskActionInstance>>| slots.into_iter().map(|x| x.map(|v| v.into_action_instance(path))).collect();
		let mut pages: Vec<ProfilePage> = if self.pages.is_empty() {
			vec![ProfilePage {
				keys: map_slots(self.keys),
				sliders: map_slots(self.sliders),
				infobars: map_slots(self.infobars),
			}]
		} else {
			self.pages
				.into_iter()
				.map(|page| ProfilePage {
					keys: map_slots(page.keys),
					sliders: map_slots(page.sliders),
					infobars: map_slots(page.infobars),
				})
				.collect()
		};

		for (index, page) in pages.iter_mut().enumerate() {
			let page_index = index as u8;
			for slot in page.slots_mut().filter_map(|slot| slot.as_mut()) {
				slot.context.page = page_index;
				if let Some(children) = &mut slot.children {
					for child in children {
						child.context.page = page_index;
					}
				}
			}
		}

		let mut profile = Profile {
			id,
			current_page: self.current_page.min(pages.len().saturating_sub(1)),
			pages,
			keys: Vec::new(),
			sliders: Vec::new(),
			infobars: Vec::new(),
			stale: false,
		};
		profile.hydrate_current_slots();
		profile
	}
}

impl super::FromAndIntoDiskValue for Profile {
	fn into_value(&self) -> Result<serde_json::Value, serde_json::Error> {
		let disk: DiskProfile = self.into();
		serde_json::to_value(disk)
	}
	fn from_value(value: serde_json::Value, path: &Path) -> Result<Profile, serde_json::Error> {
		let disk: DiskProfile = serde_json::from_value(value)?;
		Ok(disk.into_profile(path))
	}
}
