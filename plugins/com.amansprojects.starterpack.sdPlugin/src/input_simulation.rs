use openaction::*;

use enigo::{
	agent::{Agent, Token},
	Enigo, Settings,
};

pub fn key_down(event: KeyEvent) -> EventHandlerResult {
	if let Some(value) = event.payload.settings.as_object().unwrap().get("down") {
		let value = value.as_str().unwrap();
		if value.trim().is_empty() {
			return Ok(());
		}
		let mut enigo = Enigo::new(&Settings::default())?;
		let tokens: Vec<Token> = ron::from_str(value)?;
		for token in tokens {
			enigo.execute(&token).unwrap();
		}
	}

	Ok(())
}

pub fn key_up(event: KeyEvent) -> EventHandlerResult {
	if let Some(value) = event.payload.settings.as_object().unwrap().get("up") {
		let value = value.as_str().unwrap();
		if value.trim().is_empty() {
			return Ok(());
		}
		let mut enigo = Enigo::new(&Settings::default())?;
		let tokens: Vec<Token> = ron::from_str(value)?;
		for token in tokens {
			enigo.execute(&token).unwrap();
		}
	}

	Ok(())
}
