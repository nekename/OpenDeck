use crate::shared::{Action, ActionInstance, config_dir};
use crate::store::profiles::{acquire_locks_mut, get_slot_mut};

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use image::imageops::overlay;
use image::{DynamicImage, Rgba, RgbaImage};
use log::{trace, warn};
use serde_json::{Map, Value};
use streamdeck_strip_render::get_incremental_renderer_with_size;
use streamdeck_strip_render::layout::{LayoutItem, PixmapSource};

/// Check if the specified controller supports dynamic layouts.
///
/// Only `"Encoder"` and `"Neo"` use dynamic layout templates via `streamdeck_strip_render`.
/// Standard `"Keypad"` does not support dynamic layouts.
pub fn supports_dynamic_layout(controller: &str) -> bool {
	matches!(controller, "Encoder" | "Neo")
}

pub fn initialise_layout(action: &mut Action, controller: &str, layout: Option<String>) -> Result<(), anyhow::Error> {
	let is_neo = controller == "Neo";
	let (width, height) = if is_neo {
		(232, 50)
	} else {
		(200, 100)
	};

	let load_layout = if is_neo {
		match layout {
			Some(s) if !s.is_empty() => s,
			_ => {
				action.layout_parsed = None;
				return Ok(());
			}
		}
	} else {
		let layout_default = action.encoder.as_ref().map(|e| e.layout.clone());
		match layout.or(layout_default) {
			Some(s) if !s.is_empty() => s,
			_ => {
				if action.encoder.is_some() {
					"$X1".to_string()
				} else {
					action.layout_parsed = None;
					return Ok(());
				}
			}
		}
	};

	let layout = if load_layout.starts_with('$') {
		load_layout.clone()
	} else {
		let plugin_dir = config_dir().join("plugins").join(&action.plugin);
		let layout_file = plugin_dir.join(&load_layout);

		let Ok(plugin_dir) = plugin_dir.canonicalize() else {
			bail!("Failed to canonicalize plugin directory: {}", plugin_dir.display());
		};

		match layout_file.canonicalize() {
			Ok(resolved) if resolved.starts_with(&plugin_dir) => resolved.to_string_lossy().into_owned(),
			Ok(_) => {
				action.layout_parsed = None;
				bail!("Encoder layout path escapes plugin directory: {}", load_layout);
			}
			Err(error) => {
				action.layout_parsed = None;
				bail!("Failed to canonicalize encoder layout path {}: {}", load_layout, error);
			}
		}
	};

	match load_encoder_layout(&layout) {
		Ok(parsed) => action.layout_parsed = Some(get_incremental_renderer_with_size(parsed, None, width, height)?),
		Err(error) => {
			action.layout_parsed = None;
			bail!("Failed to load encoder layout {}: {}", load_layout, error)
		}
	}
	Ok(())
}

fn load_encoder_layout(layout: &str) -> Result<serde_json::Value> {
	match layout {
		"$A0" => Ok(serde_json::from_str(include_str!("../../static/encoder_layouts/A0.json"))?),
		"$A1" => Ok(serde_json::from_str(include_str!("../../static/encoder_layouts/A1.json"))?),
		"$B1" => Ok(serde_json::from_str(include_str!("../../static/encoder_layouts/B1.json"))?),
		"$B2" => Ok(serde_json::from_str(include_str!("../../static/encoder_layouts/B2.json"))?),
		"$C1" => Ok(serde_json::from_str(include_str!("../../static/encoder_layouts/C1.json"))?),
		"$X1" => Ok(serde_json::from_str(include_str!("../../static/encoder_layouts/X1.json"))?),

		path_str => {
			if path_str.is_empty() {
				bail!("Encoder layout path is blank, ignoring");
			}

			// This doesn't match an existing built-in layout
			if path_str.starts_with('$') {
				bail!("Invalid encoder layout type: {}", path_str);
			}

			let path = Path::new(path_str);
			if !path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json")) {
				bail!("Invalid encoder layout type: {}", path_str);
			}

			let content = std::fs::read_to_string(path)?;
			Ok(serde_json::from_str(&content)?)
		}
	}
}

pub async fn generate_encoder_image(context: &crate::shared::Context, fallback: &[u8]) -> Result<DynamicImage, anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	let slot = get_slot_mut(context, &mut locks).await?;

	let img = if let Some(instance) = slot {
		if instance.action.layout_parsed.is_some() {
			let result = get_layout_image(instance).context("Failed to render encoder image");
			Some(result?)
		} else {
			None
		}
	} else {
		None
	};
	drop(locks);

	match img {
		Some(img) => Ok(img),
		None => {
			// If we get here, this is either an Encoder action that doesn't have an Encoder config in the manifest, or we were
			// unable to locate the instance for this action. This realistically shouldn't happen, but if it does, we'll fall back
			// to rendering what was provided to this function call.
			trace!("No encoder instance / config found for action; using fallback image");

			let mut fallback_canvas = RgbaImage::from_pixel(200, 100, Rgba([0, 0, 0, 255]));
			let fallback_img = image::load_from_memory(fallback)
				.context("Failed to decode fallback image")?
				.resize(72, 72, image::imageops::FilterType::Nearest);

			overlay(&mut fallback_canvas, &fallback_img.to_rgba8(), 64, 14);

			Ok(DynamicImage::ImageRgba8(fallback_canvas))
		}
	}
}

pub async fn generate_infobar_image(context: &crate::shared::Context, fallback: &[u8]) -> Result<DynamicImage, anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	let slot = get_slot_mut(context, &mut locks).await?;

	let img = if let Some(instance) = slot {
		if instance.context.controller == "Neo" && instance.action.layout_parsed.is_some() {
			let rendered = get_layout_image(instance).context("Failed to render infobar layout")?;
			// Stream Deck Neo layout canvas is 232x50, physical display is 248x58.
			// Center the 232x50 layout onto the 248x58 canvas with 8px horizontal and 4px vertical margin.
			let mut canvas = RgbaImage::from_pixel(248, 58, Rgba([0, 0, 0, 255]));
			overlay(&mut canvas, &rendered.to_rgba8(), 8, 4);
			Some(DynamicImage::ImageRgba8(canvas))
		} else {
			None
		}
	} else {
		None
	};
	drop(locks);

	match img {
		Some(img) => Ok(img),
		None => {
			let fallback_img = image::load_from_memory(fallback)
				.context("Failed to decode fallback image")?
				.resize_exact(248, 58, image::imageops::FilterType::Lanczos3);
			Ok(fallback_img)
		}
	}
}

fn get_layout_image(instance: &mut ActionInstance) -> Result<DynamicImage, anyhow::Error> {
	let is_neo = instance.context.controller == "Neo";
	let (w, h) = if is_neo { (232, 50) } else { (200, 100) };
	let Some(ref mut renderer) = instance.action.layout_parsed else {
		// Something's gone horribly wrong here; we should have a layout. Render a blank image.
		return Ok(DynamicImage::new_rgb8(w, h));
	};

	let path = config_dir().join("plugins").join(&instance.action.plugin);
	let path_canonical = path.canonicalize()?;

	let state_idx = instance.current_state as usize;

	// Override the title if it's set in the state.
	let override_title = {
		let t = instance.states[state_idx].text.trim();
		(!t.is_empty()).then(|| t.to_string())
	};

	// Similar to the title, except for the rendered icon (we check whether the state matches the default, and if it doesn't, we use it).
	let state_image = &instance.states[state_idx].image.trim();

	let override_icon = {
		let default_image = &instance.action.states[state_idx].image.trim();
		(!state_image.is_empty() && state_image != default_image).then(|| state_image.to_string())
	};

	// We need to do small item corrections here.
	let mut feedback = Map::new();
	let layout = renderer.layout();

	// If the layout doesn't have a title, fall back to the action title.
	if let Some(title) = layout.item("title")
		&& let LayoutItem::Text(title) = title
		&& title.value.as_deref().is_none_or(str::is_empty)
	{
		feedback.insert("title".to_string(), Value::String(instance.action.name.clone()));
	}

	// If the layout doesn't have an icon, fall back to the state image.
	if let Some(icon) = layout.item("icon")
		&& let LayoutItem::Pixmap(icon) = icon
		&& icon.value == PixmapSource::None
	{
		feedback.insert("icon".to_string(), Value::String(state_image.to_string()));
	}

	// Expand and sandbox every pixmap item's relative file path.
	for item in &layout.items {
		let LayoutItem::Pixmap(p) = item else { continue };
		let PixmapSource::File(v) = &p.value else { continue };
		if v.is_empty() {
			continue;
		}

		let resolved = {
			// We need to make sure this path isn't already canonical.
			let candidate = if Path::new(v).is_absolute() { PathBuf::from(v) } else { path.join(v) };
			match candidate.canonicalize() {
				Ok(resolved) if resolved.starts_with(&path_canonical) => resolved.to_string_lossy().into_owned(),
				Ok(resolved) => {
					warn!("Attempted to load image outside of base path: {resolved:?}");
					String::new()
				}
				Err(_) => {
					warn!("Unable to canonicalize path: {candidate:?}");
					String::new()
				}
			}
		};

		feedback.insert(item.key().to_string(), Value::String(resolved));
	}

	// Send changes to the renderer; note that the overrides are NOOP if they haven't changed.
	renderer.set_icon_override(override_icon);
	renderer.set_title_override(override_title);
	renderer.set_feedback(Value::Object(feedback))?;
	Ok(DynamicImage::ImageRgba8(renderer.get_image()))
}
