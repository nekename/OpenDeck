use super::{Coordinates, send_to_plugin};

use crate::shared::{ActionContext, ActionInstance};

use serde::Serialize;

#[derive(Serialize)]
struct TitleParametersDidChangeEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: TitleParametersDidChangePayload,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct TitleParametersDidChangePayload {
	settings: serde_json::Value,
	coordinates: Coordinates,
	state: u16,
	title: String,
	titleParameters: TitleParameters,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct TitleParameters {
	fontFamily: String,
	fontSize: u16,
	fontStyle: String,
	fontUnderline: bool,
	showTitle: bool,
	titleAlignment: String,
	titleColor: String,
}

pub async fn title_parameters_did_change(instance: &ActionInstance, state: u16) -> Result<(), anyhow::Error> {
	let instance = instance.clone();
	let state = instance.states[state as usize].clone();

	send_to_plugin(
		&instance.action.plugin,
		&TitleParametersDidChangeEvent {
			event: "titleParametersDidChange",
			action: instance.action.uuid,
			context: instance.context.clone(),
			device: instance.context.device,
			payload: TitleParametersDidChangePayload {
				settings: instance.settings,
				coordinates: Coordinates {
					row: instance.context.position / 3,
					column: instance.context.position % 3,
				},
				state: instance.current_state,
				title: state.text,
				titleParameters: TitleParameters {
					fontFamily: String::new(),
					fontSize: state.size.0,
					fontStyle: state.style,
					fontUnderline: state.underline,
					showTitle: state.show,
					titleAlignment: state.alignment,
					titleColor: state.colour,
				},
			},
		},
	)
	.await
}
