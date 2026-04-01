import type { ActionState } from "./ActionState.ts";
import type { Context } from "./Context.ts";

import { getWebserverUrl } from "./ports.ts";

import { invoke } from "@tauri-apps/api/core";

/** Check if an image source is potentially animated (GIF or WebP). */
function isAnimatedSource(src: string): boolean {
	if (src.startsWith("data:image/gif")) return true;
	if (src.startsWith("data:image/webp")) return true;
	if (!src.startsWith("data:")) {
		const lower = src.toLowerCase();
		if (lower.endsWith(".gif") || lower.endsWith(".webp")) return true;
	}
	return false;
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

/** Shared global origin for all animation clocks, so all animated keys on the same page stay in sync. */
const ANIMATION_ORIGIN = performance.now();

/** Compute which frame index to display given a shared time origin and per-frame durations. */
function computeFrameIndex(durations: number[], now: number): number {
	// Total loop duration
	let totalDuration = 0;
	for (const d of durations) totalDuration += d;
	if (totalDuration <= 0) return 0;

	// Time elapsed within the current loop iteration
	let elapsed = (now - ANIMATION_ORIGIN) % totalDuration;

	for (let i = 0; i < durations.length; i++) {
		if (elapsed < durations[i]) return i;
		elapsed -= durations[i];
	}
	return 0;
}

export class CanvasLock {
	currentLock = Promise.resolve();
	async lock() {
		let unlockNext: () => void;
		const willLock = new Promise<void>((resolve) => unlockNext = resolve);
		const previousLock = this.currentLock;
		this.currentLock = willLock;
		await previousLock;
		return unlockNext!;
	}
}

/** Draw one composited frame onto the canvas (background + image + text + overlays + press). */
function drawFrame(
	canvas: HTMLCanvasElement,
	context: CanvasRenderingContext2D,
	image: CanvasImageSource,
	state: ActionState,
	showOk: boolean,
	showAlert: boolean,
	pressed: boolean,
	rotation: number | undefined,
	scale: number,
	okImage: HTMLImageElement | null,
	alertImage: HTMLImageElement | null,
) {
	context.save();
	if (rotation) {
		context.translate(canvas.width / 2, canvas.height / 2);
		context.rotate(rotation * Math.PI / 180);
		context.translate(-canvas.width / 2, -canvas.height / 2);
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

	// Draw text
	if (state.show) {
		const size = state.size * 2 * scale;
		context.textAlign = "center";
		context.font = (state.style.includes("Bold") ? "bold " : "") + (state.style.includes("Italic") ? "italic " : "") +
			`${size}px "${state.family}", sans-serif`;
		context.fillStyle = state.colour;
		context.strokeStyle = state.stroke_colour;
		context.lineWidth = state.stroke_size * scale;
		context.textBaseline = "top";
		const x = canvas.width / 2;
		let y = canvas.height / 2 - (size * state.text.split("\n").length * 0.5);
		switch (state.alignment) {
			case "top":
				y = context.lineWidth;
				break;
			case "bottom":
				y = canvas.height - (size * state.text.split("\n").length) - context.lineWidth;
				break;
		}
		for (const [index, line] of Object.entries(state.text.split("\n"))) {
			context.strokeText(line, x, y + (size * parseInt(index)));
			context.fillText(line, x, y + (size * parseInt(index)));
			if (state.underline) {
				const width = context.measureText(line).width;
				context.fillStyle = "black";
				context.fillRect(x - (width / 2) - 3, y + (size * parseInt(index)) + size, width + 6, 9);
				context.fillStyle = state.colour;
				context.fillRect(x - (width / 2), y + (size * parseInt(index)) + size + 4, width, 3);
			}
		}
	}

	if (showOk && okImage) {
		context.drawImage(okImage, 0, 0, canvas.width, canvas.height);
	}

	if (showAlert && alertImage) {
		context.drawImage(alertImage, 0, 0, canvas.width, canvas.height);
	}

	// Make the image smaller while the button is pressed.
	if (pressed) {
		const smallCanvas = document.createElement("canvas");
		smallCanvas.width = canvas.width;
		smallCanvas.height = canvas.height;
		const newContext = smallCanvas.getContext("2d");
		const margin = 0.1;
		if (newContext) {
			newContext.drawImage(
				canvas,
				canvas.width * margin,
				canvas.height * margin,
				canvas.width * (1 - (margin * 2)),
				canvas.height * (1 - (margin * 2)),
			);
			context.clearRect(0, 0, canvas.width, canvas.height);
			context.drawImage(smallCanvas, 0, 0);
		}
	}

	context.restore();
}

/** Pre-load a static overlay image (ok/alert). Returns null if it fails to load. */
function loadOverlay(src: string): Promise<HTMLImageElement | null> {
	const img = document.createElement("img");
	img.crossOrigin = "anonymous";
	img.src = src;
	return new Promise((resolve) => {
		img.onload = () => resolve(img);
		img.onerror = () => resolve(null);
	});
}

interface AnimatedFrames {
	frames: ImageBitmap[];
	durations: number[]; // in milliseconds
}

/** Decode individual frames from an animated GIF/WebP using the ImageDecoder WebCodecs API. */
async function decodeAnimatedFrames(src: string): Promise<AnimatedFrames | null> {
	// deno-lint-ignore no-explicit-any
	if (!("ImageDecoder" in globalThis)) return null;

	try {
		const response = await fetch(src);
		const blob = await response.blob();
		const type = blob.type || (src.startsWith("data:") ? src.substring(5, src.indexOf(";")) : "");
		if (!type) return null;

		// deno-lint-ignore no-explicit-any
		const decoder = new (globalThis as any).ImageDecoder({ type, data: blob.stream() });
		await decoder.completed;

		const track = decoder.tracks.selectedTrack;
		if (!track || track.frameCount <= 1) {
			decoder.close();
			return null;
		}

		const frames: ImageBitmap[] = [];
		const durations: number[] = [];

		for (let i = 0; i < track.frameCount; i++) {
			const result = await decoder.decode({ frameIndex: i });
			const bitmap = await createImageBitmap(result.image);
			frames.push(bitmap);
			// VideoFrame.duration is in microseconds; default to 100ms if missing
			durations.push((result.image.duration || 100000) / 1000);
			result.image.close();
		}

		decoder.close();
		return { frames, durations };
	} catch {
		return null;
	}
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
	preview?: boolean,
): Promise<(() => void) | string | undefined> {
	// Create canvas
	let scale = 1;
	if (!canvas) {
		canvas = document.createElement("canvas");
		canvas.width = 144;
		canvas.height = 144;
	} else {
		scale = canvas.width / 144;
	}

	const context = canvas.getContext("2d");
	if (!context) return;

	const imageSrc = processImage ? getImage(state.image, fallback) : state.image;
	const animated = isAnimatedSource(imageSrc);

	// Pre-load overlay images if needed
	const okImage = showOk ? await loadOverlay("/ok.png") : null;
	const alertImage = showAlert ? await loadOverlay("/alert.png") : null;

	try {
		// Load image
		const image = document.createElement("img");
		image.crossOrigin = "anonymous";
		image.src = imageSrc;
		if (image.src == undefined) return;
		await new Promise((resolve, reject) => {
			image.onload = resolve;
			image.onerror = reject;
		});

		if (animated && !preview) {
			// Try to decode individual frames using the ImageDecoder WebCodecs API.
			const animData = await decodeAnimatedFrames(imageSrc);
			if (animData) {
				let stopped = false;
				let deviceUpdateInFlight = false;
				const DEVICE_MIN_INTERVAL = 150; // minimum ms between device updates

				// Separate canvas for device frames to avoid interfering with UI.
				const deviceCanvas = document.createElement("canvas");
				deviceCanvas.width = canvas!.width;
				deviceCanvas.height = canvas!.height;
				const deviceCtx = deviceCanvas.getContext("2d")!;

				const tick = () => {
					if (stopped) return;
					const now = performance.now();

					// All animations use the shared global clock so they stay in sync.
					const frameIndex = computeFrameIndex(animData.durations, now);

					// Draw current frame to the visible canvas (UI)
					drawFrame(canvas!, context, animData.frames[frameIndex], state, showOk, showAlert, pressed, rotation, scale, okImage, alertImage);

					// Send the same frame to the device at a throttled rate
					if (active && slotContext && !deviceUpdateInFlight) {
						deviceUpdateInFlight = true;
						drawFrame(deviceCanvas, deviceCtx, animData.frames[frameIndex], state, showOk, showAlert, false, rotation, scale, okImage, alertImage);
						invoke("update_image", { context: slotContext, image: deviceCanvas.toDataURL("image/jpeg") })
							.finally(() => {
								setTimeout(() => {
									deviceUpdateInFlight = false;
								}, DEVICE_MIN_INTERVAL);
							});
					}

					requestAnimationFrame(tick);
				};

				requestAnimationFrame(tick);

				return () => {
					stopped = true;
					for (const frame of animData.frames) frame.close();
				};
			}
			// If ImageDecoder is unavailable or image is not truly animated, fall through to static draw.
		}

		// Static image: single draw
		drawFrame(canvas, context, image, state, showOk, showAlert, pressed, rotation, scale, okImage, alertImage);
	} catch (error: any) {
		if (!(error instanceof Event)) console.error(error);
		context.save();
		if (rotation) {
			context.translate(canvas.width / 2, canvas.height / 2);
			context.rotate(rotation * Math.PI / 180);
			context.translate(-canvas.width / 2, -canvas.height / 2);
		}
		context.clearRect(0, 0, canvas.width, canvas.height);
		context.restore();
		showAlert = true;
		const alertFallback = await loadOverlay("/alert.png");
		if (alertFallback) {
			context.drawImage(alertFallback, 0, 0, canvas.width, canvas.height);
		}
	}

	if (active && slotContext) setTimeout(async () => await invoke("update_image", { context: slotContext, image: canvas!.toDataURL("image/jpeg") }), 10);
	else if (preview) return canvas.toDataURL();
}

export async function resizeImage(source: string): Promise<string | undefined> {
	// Preserve animated formats (GIF, WebP) as-is to avoid flattening to a single frame.
	if (isAnimatedSource(source)) return source;

	const canvas = document.createElement("canvas");
	canvas.width = 288;
	canvas.height = 288;
	const context = canvas.getContext("2d");
	if (!context) return;

	const image = document.createElement("img");
	image.crossOrigin = "anonymous";
	image.src = source;
	await new Promise((resolve) => image.onload = resolve);

	let xOffset = 0, yOffset = 0;
	let xScaled = canvas.width, yScaled = canvas.height;
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
