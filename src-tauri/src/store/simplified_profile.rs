//! Duplicates of many structs to facilitate saving profiles to disk in a format that can be transferred between devices or systems.

use crate::shared::{Action, ActionContext, ActionInstance, ActionState, Profile};

use std::{
	fs,
	path::{Path, PathBuf},
};

use path_slash::{PathBufExt, PathExt};
use serde::{Deserialize, Serialize};

#[derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr)]
pub struct DiskActionContext {
	pub controller: String,
	pub position: u8,
	pub index: u16,
}

impl std::fmt::Display for DiskActionContext {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}.{}.{}", self.controller, self.position, self.index)
	}
}

impl std::str::FromStr for DiskActionContext {
	type Err = std::num::ParseIntError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let segments: Vec<&str> = s.split('.').collect();
		let mut offset: usize = 0;
		if segments.len() == 5 {
			offset = 2;
		}
		let controller = segments[offset].to_owned();
		let position = u8::from_str(segments[1 + offset])?;
		let index = u16::from_str(segments[2 + offset])?;
		Ok(Self { controller, position, index })
	}
}

impl From<ActionContext> for DiskActionContext {
	fn from(value: ActionContext) -> Self {
		Self {
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

		for (index, state) in value.states.iter_mut().enumerate() {
			if state.image.trim() == "data:" {
				state.image = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIW2NgYGD4DwABBAEAwS2OUAAAAABJRU5ErkJggg==".to_owned();
			}

			if state.image.starts_with("data:") && state.image.contains('/') && state.image.contains(',') {
				let mut extension = state.image.split_once('/').unwrap().1.split_once(',').unwrap().0;
				if extension.contains(';') {
					extension = extension.split_once(';').unwrap().0;
				}
				if extension.contains('+') {
					extension = extension.split_once('+').unwrap().0;
				}

				let data = if state.image.contains(";base64,") {
					use base64::Engine;
					let Ok(data) = base64::engine::general_purpose::STANDARD.decode(state.image.split_once(";base64,").unwrap().1) else {
						continue;
					};
					data
				} else {
					state.image.split_once(',').unwrap().1.as_bytes().to_vec()
				};

				let filename = format!("{}.{}", index, extension);
				if fs::create_dir_all(&image_dir).is_err() || fs::write(image_dir.join(&filename), data).is_err() {
					continue;
				};
				state.image = filename;
			}

			state.image = normalise_path(&state.image);
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

		let mut states = self.states.clone();
		for state in states.iter_mut() {
			if let Some(true) = state.image.chars().next().map(|v| v.is_numeric()) {
				state.image = config_dir
					.join("images")
					.join(&device)
					.join(&profile)
					.join(self.context.to_string())
					.join(&state.image)
					.to_string_lossy()
					.into_owned();
			} else {
				state.image = reconstruct_path(&state.image);
			}
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
pub struct DiskProfile {
	pub keys: Vec<Option<DiskActionInstance>>,
	pub sliders: Vec<Option<DiskActionInstance>>,
}

impl From<&Profile> for DiskProfile {
	fn from(value: &Profile) -> Self {
		Self {
			keys: value.keys.clone().into_iter().map(|x| x.map(|v| v.into())).collect(),
			sliders: value.sliders.clone().into_iter().map(|x| x.map(|v| v.into())).collect(),
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
		Profile {
			id,
			keys: self.keys.into_iter().map(|x| x.map(|v| v.into_action_instance(path))).collect(),
			sliders: self.sliders.into_iter().map(|x| x.map(|v| v.into_action_instance(path))).collect(),
		}
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
