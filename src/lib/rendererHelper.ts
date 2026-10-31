import type { ActionState } from "./ActionState.ts";
import type { Context } from "./Context.ts";

import { getWebserverUrl } from "./ports.ts";

import { invoke } from "@tauri-apps/api/core";

let configDirPromise: Promise<string> | undefined;

function configDir(): Promise<string> {
	configDirPromise ??= invoke<string>("get_config_dir");
	return configDirPromise;
}

// Saved button images are stored as bare filenames ("0.png"). The action icon for those
// buttons is often a blank loading tile, so the filename has to be resolved to the image file.
function bareSavedFile(image: string | undefined): image is string {
	return !!image && !image.startsWith("data:") && !image.startsWith("opendeck/") && !image.includes("/") && !image.includes("\\");
}

// Companion's appear/disappear placeholder is a ~140 byte data URL. A real button image is a few KB.
function tinyDataImage(image: string | undefined): boolean {
	if (!image?.startsWith("data:")) return false;
	const b64 = image.split(";base64,")[1];
	if (!b64) return image.length < 400;
	return b64.length < 360;
}

function useSavedArt(image: string | undefined, fallback: string | undefined): boolean {
	if (!image || tinyDataImage(image)) return true;
	if (!image.startsWith("data:") && !image.startsWith("opendeck/") && !image.includes("/") && !image.includes("\\")) return true;
	if (fallback && image === fallback) return true;
	return image.includes("loadingIcon");
}

async function resolveSavedImage(image: string, slotContext: Context, instanceContext?: string): Promise<string> {
	const root = await configDir();
	const index = instanceContext?.split(".").pop();
	const slotIndex = index && /^\d+$/.test(index) ? index : "0";
	const disk =
		slotContext.page > 0
			? `${slotContext.page}.${slotContext.controller}.${slotContext.position}.${slotIndex}`
			: `${slotContext.controller}.${slotContext.position}.${slotIndex}`;
	return getWebserverUrl(`${root}/images/${slotContext.device}/${slotContext.profile}/${disk}/${image}`);
}

export function getImage(image: string | undefined, fallback: string | undefined): string {
	if (!image) return fallback ? getImage(fallback, undefined) : "/alert.png";
	if (image.startsWith("opendeck/")) return image.replace("opendeck", "");
	if (!image.startsWith("data:")) return getWebserverUrl(image);
	const svgxmlre = /^data:image\/svg\+xml(?!.*?;base64.*?)(?:;[\w=]*)*,(.+)/;
	const base64re = /^data:image\/(apng|avif|gif|jpeg|png|svg\+xml|webp|bmp|x-icon|tiff);base64,([A-Za-z0-9+/]+={0,2})?/;
	if (svgxmlre.test(image)) {
		let svg = (svgxmlre.exec(image) as RegExpExecArray)[1].replace(/\;$/, "");
		try {
			svg = decodeURIComponent(svg);
		} finally {
			image = "data:image/svg+xml," + encodeURIComponent(svg);
		}
	}
	if (base64re.test(image)) {
		const exec = base64re.exec(image)!;
		if (!exec[2]) return fallback ? getImage(fallback, undefined) : "/alert.png";
		else image = exec[0];
	}
	return image;
}

export class CanvasLock {
	currentLock = Promise.resolve();
	async lock() {
		let unlockNext: () => void;
		const willLock = new Promise<void>((resolve) => (unlockNext = resolve));
		const previousLock = this.currentLock;
		this.currentLock = willLock;
		await previousLock;
		return unlockNext!;
	}
}

// Handlers must be attached before src. A cached image can fire load synchronously, before a handler set afterwards, and the draw then hangs with a blank canvas.
export function loadImage(src: string): Promise<HTMLImageElement> {
	return new Promise((resolve, reject) => {
		const image = document.createElement("img");
		image.crossOrigin = "anonymous";
		let settled = false;
		const succeed = () => {
			if (settled) return;
			settled = true;
			resolve(image);
		};
		const fail = () => {
			if (settled) return;
			settled = true;
			reject(new Event("error"));
		};
		image.onload = succeed;
		image.onerror = fail;
		image.src = src;
		if (image.complete && image.naturalWidth > 0) succeed();
	});
}

export async function renderImage(
	canvas: HTMLCanvasElement | null,
	slotContext: Context | null,
	state: ActionState,
	fallback: string | undefined,
	showOk: boolean,
	showAlert: boolean,
	processImage: boolean,
	active: boolean,
	pressed: boolean,
	rotation?: number,
	stillCurrent?: () => boolean,
	instanceContext?: string,
	stateIndex = 0,
) {
	const live = () => !stillCurrent || stillCurrent();
	// Create canvas
	let scale = 1;
	if (!canvas) {
		canvas = document.createElement("canvas");
		canvas.width = 144;
		canvas.height = 144;
	} else {
		// Use height for scale to handle rectangular infobar canvases
		scale = canvas.height / 144;
	}

	const context = canvas.getContext("2d");
	if (!context) return;

	context.save();
	if (rotation) {
		context.translate(canvas.width / 2, canvas.height / 2);
		context.rotate((rotation * Math.PI) / 180);
		context.translate(-canvas.width / 2, -canvas.height / 2);
	}

	try {
		const imageSource = state.image_override ?? state.image;
		let image: HTMLImageElement | undefined;
		// A saved "0.png" (or the loading-icon placeholder that replaced it) must paint the file on disk.
		if (processImage && slotContext && !state.image_override && useSavedArt(state.image, fallback)) {
			const file = bareSavedFile(state.image) ? state.image : `${stateIndex}.png`;
			try {
				image = await loadImage(await resolveSavedImage(file, slotContext, instanceContext));
			} catch {
				image = undefined;
			}
		}
		if (!image) {
			const display = tinyDataImage(imageSource) ? undefined : imageSource;
			const imageSrc = processImage ? getImage(display, fallback) : display;
			if (imageSrc == undefined) {
				context.restore();
				return;
			}
			image = await loadImage(imageSrc);
		}
		if (!live()) {
			context.restore();
			return;
		}

		context.clearRect(0, 0, canvas.width, canvas.height);

		// Draw background color
		if (!state.background_colour.startsWith("#000000")) {
			context.fillStyle = state.background_colour;
			context.fillRect(0, 0, canvas.width, canvas.height);
		}

		// Draw image
		context.imageSmoothingQuality = "high";
		const imageScale = Math.max(10, state.image_scale || 100) / 100;
		const xScaled = canvas.width * imageScale;
		const yScaled = canvas.height * imageScale;
		const xOffset = (canvas.width - xScaled) / 2;
		const yOffset = (canvas.height - yScaled) / 2;
		context.drawImage(image, xOffset, yOffset, xScaled, yScaled);
	} catch (error: any) {
		if (!live()) {
			context.restore();
			return;
		}
		if (!(error instanceof Event)) console.error(error);
		context.clearRect(0, 0, canvas.width, canvas.height);
		showAlert = true;
	}

	if (!live()) {
		context.restore();
		return;
	}

	// Draw text
	if (state.show) {
		const textSource = state.text_override || state.text;
		const size = state.size * 2 * scale;
		context.textAlign = "center";
		context.font =
			(state.style.includes("Bold") ? "bold " : "") + (state.style.includes("Italic") ? "italic " : "") + `${size}px "${state.family}", sans-serif`;
		context.fillStyle = state.colour;
		context.strokeStyle = state.stroke_colour;
		context.lineWidth = state.stroke_size * scale;
		context.textBaseline = "top";
		const x = canvas.width / 2;
		let y = canvas.height / 2 - size * textSource.split("\n").length * 0.5;
		switch (state.alignment) {
			case "top":
				y = context.lineWidth;
				break;
			case "bottom":
				y = canvas.height - size * textSource.split("\n").length - context.lineWidth;
				break;
		}
		for (const [index, line] of Object.entries(textSource.split("\n"))) {
			context.strokeText(line, x, y + size * parseInt(index));
			context.fillText(line, x, y + size * parseInt(index));
			if (state.underline) {
				const width = context.measureText(line).width;
				// Set to black for the outline, since it uses the same fill style info as the text colour.
				context.fillStyle = "black";
				context.fillRect(x - width / 2 - 3, y + size * parseInt(index) + size, width + 6, 9);
				// Reset to the user's choice of text colour.
				context.fillStyle = state.colour;
				context.fillRect(x - width / 2, y + size * parseInt(index) + size + 4, width, 3);
			}
		}
	}

	if (!live()) {
		context.restore();
		return;
	}

	if (showOk) {
		try {
			const okImage = await loadImage("/ok.png");
			if (!live()) {
				context.restore();
				return;
			}
			context.drawImage(okImage, 0, 0, canvas.width, canvas.height);
		} catch {
			// Overlay is optional. Keep the button image.
		}
	}

	if (showAlert) {
		try {
			const alertImage = await loadImage("/alert.png");
			if (!live()) {
				context.restore();
				return;
			}
			context.drawImage(alertImage, 0, 0, canvas.width, canvas.height);
		} catch {
			// Overlay is optional. Keep the button image.
		}
	}

	// Make the image smaller while the button is pressed.
	if (pressed) {
		const smallCanvas = document.createElement("canvas");
		smallCanvas.width = canvas.width;
		smallCanvas.height = canvas.height;
		const newContext = smallCanvas.getContext("2d");
		const margin = 0.1;
		if (newContext) {
			newContext.drawImage(canvas, canvas.width * margin, canvas.height * margin, canvas.width * (1 - margin * 2), canvas.height * (1 - margin * 2));
			context.clearRect(0, 0, canvas.width, canvas.height);
			context.drawImage(smallCanvas, 0, 0);
		}
	}

	context.restore();

	if (active && slotContext && live())
		setTimeout(async () => {
			if (!live()) return;
			await invoke("update_image", { context: slotContext, image: canvas.toDataURL("image/jpeg") });
		}, 10);
}

export async function resizeImage(source: string): Promise<string | undefined> {
	const canvas = document.createElement("canvas");
	canvas.width = 288;
	canvas.height = 288;
	const context = canvas.getContext("2d");
	if (!context) return;

	const image = await loadImage(source);

	let xOffset = 0,
		yOffset = 0;
	let xScaled = canvas.width,
		yScaled = canvas.height;
	if (image.width > image.height) {
		const ratio = image.height / image.width;
		yScaled = canvas.height * ratio;
		yOffset = (canvas.height - yScaled) / 2;
	} else if (image.width < image.height) {
		const ratio = image.width / image.height;
		xScaled = canvas.width * ratio;
		xOffset = (canvas.width - xScaled) / 2;
	}

	context.imageSmoothingQuality = "high";
	context.clearRect(0, 0, canvas.width, canvas.height);
	context.drawImage(image, xOffset, yOffset, xScaled, yScaled);

	return canvas.toDataURL();
}
