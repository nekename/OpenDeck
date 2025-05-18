pub(crate) mod devices;
mod misc;
mod settings;
mod states;

use crate::{
	shared::ActionContext,
	store::profiles::{acquire_locks, get_instance},
};

use tokio_tungstenite::tungstenite::{Error, Message};

use log::warn;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "event")]
#[serde(rename_all = "camelCase")]
pub enum RegisterEvent {
	RegisterPlugin { uuid: String },
	RegisterPropertyInspector { uuid: String },
}

#[derive(Deserialize)]
pub struct ContextEvent<C = ActionContext> {
	pub context: C,
}

#[derive(Deserialize)]
pub struct PayloadEvent<T> {
	pub payload: T,
}

#[derive(Deserialize)]
pub struct ContextAndPayloadEvent<T, C = ActionContext> {
	pub context: C,
	pub payload: T,
}

#[derive(Deserialize)]
#[serde(tag = "event")]
#[serde(rename_all = "camelCase")]
pub enum InboundEventType {
	RegisterDevice(PayloadEvent<crate::shared::DeviceInfo>),
	DeregisterDevice(PayloadEvent<String>),
	RerenderImages(PayloadEvent<String>),
	KeyDown(PayloadEvent<devices::PressPayload>),
	KeyUp(PayloadEvent<devices::PressPayload>),
	EncoderChange(PayloadEvent<devices::TicksPayload>),
	EncoderDown(PayloadEvent<devices::PressPayload>),
	EncoderUp(PayloadEvent<devices::PressPayload>),
	SetSettings(ContextAndPayloadEvent<serde_json::Value>),
	GetSettings(ContextEvent),
	SetGlobalSettings(ContextAndPayloadEvent<serde_json::Value, String>),
	GetGlobalSettings(ContextEvent<String>),
	OpenUrl(PayloadEvent<misc::OpenUrlEvent>),
	LogMessage(PayloadEvent<misc::LogMessageEvent>),
	SetTitle(ContextAndPayloadEvent<states::SetTitlePayload>),
	SetImage(ContextAndPayloadEvent<states::SetImagePayload>),
	SetState(ContextAndPayloadEvent<states::SetStatePayload>),
	ShowAlert(ContextEvent),
	ShowOk(ContextEvent),
	SendToPropertyInspector(ContextAndPayloadEvent<serde_json::Value>),
	SendToPlugin(ContextAndPayloadEvent<serde_json::Value>),
	SwitchProfile(misc::SwitchProfileEvent),
	DeviceBrightness(misc::DeviceBrightnessEvent),
}

pub async fn process_incoming_message(data: Result<Message, Error>, uuid: &str) {
	if let Ok(Message::Text(text)) = data {
		let decoded: InboundEventType = match serde_json::from_str(&text) {
			Ok(event) => event,
			Err(_) => return,
		};

		if let Some(context) = match &decoded {
			InboundEventType::SetSettings(event) => Some(&event.context),
			InboundEventType::GetSettings(event) => Some(&event.context),
			InboundEventType::SetTitle(event) => Some(&event.context),
			InboundEventType::SetImage(event) => Some(&event.context),
			InboundEventType::SetState(event) => Some(&event.context),
			InboundEventType::ShowAlert(event) => Some(&event.context),
			InboundEventType::ShowOk(event) => Some(&event.context),
			InboundEventType::SendToPropertyInspector(event) => Some(&event.context),
			_ => None,
		} {
			if let Ok(Some(instance)) = get_instance(context, &acquire_locks().await).await {
				if instance.action.plugin != uuid {
					return;
				}
			} else {
				return;
			}
		} else if let InboundEventType::SetGlobalSettings(event) = &decoded {
			if event.context != uuid {
				return;
			}
		} else if let InboundEventType::GetGlobalSettings(event) = &decoded {
			if event.context != uuid {
				return;
			}
		} else if matches!(decoded, InboundEventType::SwitchProfile(_) | InboundEventType::DeviceBrightness(_)) && uuid != "com.amansprojects.starterpack.sdPlugin" {
			return;
		}

		if let Err(error) = match decoded {
			InboundEventType::RegisterDevice(event) => devices::register_device(uuid, event).await,
			InboundEventType::DeregisterDevice(event) => devices::deregister_device(uuid, event).await,
			InboundEventType::RerenderImages(event) => devices::rerender_images(event).await,
			InboundEventType::KeyDown(event) => devices::key_down(event).await,
			InboundEventType::KeyUp(event) => devices::key_up(event).await,
			InboundEventType::EncoderChange(event) => devices::encoder_change(event).await,
			InboundEventType::EncoderDown(event) => devices::encoder_down(event).await,
			InboundEventType::EncoderUp(event) => devices::encoder_up(event).await,
			InboundEventType::SetSettings(event) => settings::set_settings(event, false).await,
			InboundEventType::GetSettings(event) => settings::get_settings(event, false).await,
			InboundEventType::SetGlobalSettings(event) => settings::set_global_settings(event, false).await,
			InboundEventType::GetGlobalSettings(event) => settings::get_global_settings(event, false).await,
			InboundEventType::OpenUrl(event) => misc::open_url(event).await,
			InboundEventType::LogMessage(event) => misc::log_message(event).await,
			InboundEventType::SetTitle(event) => states::set_title(event).await,
			InboundEventType::SetImage(event) => states::set_image(event).await,
			InboundEventType::SetState(event) => states::set_state(event).await,
			InboundEventType::ShowAlert(event) => misc::show_alert(event).await,
			InboundEventType::ShowOk(event) => misc::show_ok(event).await,
			InboundEventType::SendToPropertyInspector(event) => misc::send_to_property_inspector(event).await,
			InboundEventType::SendToPlugin(_) => Ok(()),
			InboundEventType::SwitchProfile(event) => misc::switch_profile(event).await,
			InboundEventType::DeviceBrightness(event) => misc::device_brightness(event).await,
		} {
			if !error.to_string().contains("closed connection") {
				warn!("Failed to process incoming event from plugin: {}", error);
			}
		}
	}
}

pub async fn process_incoming_message_pi(data: Result<Message, Error>, uuid: &str) {
	if let Ok(Message::Text(text)) = data {
		let decoded: InboundEventType = match serde_json::from_str(&text) {
			Ok(event) => event,
			Err(_) => return,
		};

		if let Some(context) = match &decoded {
			InboundEventType::SetSettings(event) => Some(event.context.to_string()),
			InboundEventType::GetSettings(event) => Some(event.context.to_string()),
			InboundEventType::SetGlobalSettings(event) => Some(event.context.clone()),
			InboundEventType::GetGlobalSettings(event) => Some(event.context.clone()),
			InboundEventType::SendToPlugin(event) => Some(event.context.to_string()),
			_ => None,
		} {
			if context != uuid {
				return;
			}
		}

		if let Err(error) = match decoded {
			InboundEventType::SetSettings(event) => settings::set_settings(event, true).await,
			InboundEventType::GetSettings(event) => settings::get_settings(event, true).await,
			InboundEventType::SetGlobalSettings(event) => settings::set_global_settings(event, true).await,
			InboundEventType::GetGlobalSettings(event) => settings::get_global_settings(event, true).await,
			InboundEventType::OpenUrl(event) => misc::open_url(event).await,
			InboundEventType::LogMessage(event) => misc::log_message(event).await,
			InboundEventType::SendToPlugin(event) => misc::send_to_plugin(event).await,
			_ => Ok(()),
		} {
			if !error.to_string().contains("closed connection") {
				warn!("Failed to process incoming event from property inspector: {}", error);
			}
		}
	}
}
