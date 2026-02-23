use openaction::*;
use serde::{Deserialize, Serialize};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use ab_glyph::{FontRef, PxScale};
use std::io::Cursor;
use base64::{Engine as _, engine::general_purpose};
use chrono::Local;

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct InfobarClockSettings {
    // Add settings fields here if needed
}

pub struct InfobarClockAction;

fn generate_clock_image() -> Result<String, String> {
    let mut image = RgbaImage::new(248, 58);

    // Fill background with black
    for pixel in image.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 255]);
    }

    // Load font
    let font_data = include_bytes!("../assets/fonts/Roboto-Regular.ttf");
    let font = FontRef::try_from_slice(font_data).map_err(|e| e.to_string())?;

    // Get current time and date
    let now = Local::now();
    let time_str = now.format("%H:%M:%S").to_string();
    let day_str = now.format("%A").to_string().to_uppercase();
    let day_display: String = day_str.chars().take(6).collect();
    let date_str = now.format("%m/%d").to_string();

    let text_color = Rgba([255, 255, 255, 255]);

    // Draw time (left side, vertically centered, larger font)
    let time_scale = PxScale { x: 42.0, y: 42.0 };
    draw_text_mut(&mut image, text_color, 10, 8, time_scale, &font, &time_str);

    // Draw date (right side, top)
    let date_scale = PxScale { x: 24.0, y: 24.0 };
    draw_text_mut(&mut image, text_color, 160, 5, date_scale, &font, &date_str);

    // Draw day of week (right side, below date)
    let day_scale = PxScale { x: 24.0, y: 24.0 };
    // Safely take up to 6 characters (avoids UTF-8 byte boundary panics)
    draw_text_mut(&mut image, text_color, 160, 30, day_scale, &font, &day_display);

    // Encode to PNG
    let mut buffer = Cursor::new(Vec::new());
    image.write_to(&mut buffer, image::ImageFormat::Png).map_err(|e| e.to_string())?;

    // Base64 encode
    let b64 = general_purpose::STANDARD.encode(buffer.into_inner());
    Ok(format!("data:image/png;base64,{}", b64))
}

#[async_trait]
impl Action for InfobarClockAction {
    const UUID: ActionUuid = "com.amansprojects.starterpack.infobar.clock";
    type Settings = InfobarClockSettings;

    async fn will_appear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        log::info!("Infobar clock started");
        
        let instance_id = instance.instance_id.clone();
        
        tokio::spawn(async move {
            loop {
                match generate_clock_image() {
                    Ok(b64_image) => {
                        if let Some(inst) = openaction::get_instance(instance_id.clone()).await {
                            if let Err(e) = inst.set_image(Some(b64_image), None).await {
                                log::error!("Failed to set clock image: {}", e);
                            }
                        } else {
                            // Instance is no longer available, stop the loop
                            break;
                        }
                    }
                    Err(e) => log::error!("Failed to generate clock image: {}", e),
                }
                
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
        
        Ok(())
    }

    async fn key_down(
        &self,
        _instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        // Buttons not used on infobar
        Ok(())
    }
}
