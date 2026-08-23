import type { ActionState } from "./ActionState.ts";
import type { ActionInstance } from "./ActionInstance.ts";
import type { Context } from "./Context.ts";

import { getWebserverUrl } from "./ports.ts";

import { invoke } from "@tauri-apps/api/core";

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
) {
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
		// Load image
		const image = document.createElement("img");
		image.crossOrigin = "anonymous";
		image.src = processImage ? getImage(state.image, fallback) : state.image;
		if (image.src == undefined) return;
		await new Promise((resolve, reject) => {
			image.onload = resolve;
			image.onerror = reject;
		});

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
		if (!(error instanceof Event)) console.error(error);
		context.clearRect(0, 0, canvas.width, canvas.height);
		showAlert = true;
	}

	// Draw text
	if (state.show) {
		const size = state.size * 2 * scale;
		context.textAlign = "center";
		context.font =
			(state.style.includes("Bold") ? "bold " : "") + (state.style.includes("Italic") ? "italic " : "") + `${size}px "${state.family}", sans-serif`;
		context.fillStyle = state.colour;
		context.strokeStyle = state.stroke_colour;
		context.lineWidth = state.stroke_size * scale;
		context.textBaseline = "top";
		const x = canvas.width / 2;
		let y = canvas.height / 2 - size * state.text.split("\n").length * 0.5;
		switch (state.alignment) {
			case "top":
				y = context.lineWidth;
				break;
			case "bottom":
				y = canvas.height - size * state.text.split("\n").length - context.lineWidth;
				break;
		}
		for (const [index, line] of Object.entries(state.text.split("\n"))) {
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

	if (showOk) {
		const okImage = document.createElement("img");
		okImage.crossOrigin = "anonymous";
		okImage.src = "/ok.png";
		await new Promise((resolve) => {
			okImage.onload = resolve;
		});
		context.drawImage(okImage, 0, 0, canvas.width, canvas.height);
	}

	if (showAlert) {
		const alertImage = document.createElement("img");
		alertImage.crossOrigin = "anonymous";
		alertImage.src = "/alert.png";
		await new Promise((resolve) => {
			alertImage.onload = resolve;
		});
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
			newContext.drawImage(canvas, canvas.width * margin, canvas.height * margin, canvas.width * (1 - margin * 2), canvas.height * (1 - margin * 2));
			context.clearRect(0, 0, canvas.width, canvas.height);
			context.drawImage(smallCanvas, 0, 0);
		}
	}

	context.restore();

	if (active && slotContext) setTimeout(async () => await invoke("update_image", { context: slotContext, image: canvas.toDataURL("image/jpeg") }), 10);
}

function drawDotIndicator(ctx: CanvasRenderingContext2D, totalItems: number, selectedIndex: number) {
	if (totalItems <= 1) return;

	const maxDots = 9;
	const dotCount = Math.min(totalItems, maxDots);

	// Compute the scroll window — which slice of items the dots represent.
	let windowStart: number;
	if (totalItems <= maxDots) {
		// All items fit — no scrolling needed
		windowStart = 0;
	} else {
		const scrollThreshold = totalItems <= 10 ? 8 : 7;
		const scrollBackThreshold = totalItems <= 10 ? 2 : 3;
		if (selectedIndex >= scrollThreshold) {
			// Scroll right so selected is near the right end of the window
			windowStart = Math.min(selectedIndex - scrollThreshold + 1, totalItems - dotCount);
		} else if (selectedIndex < scrollBackThreshold) {
			// Scroll left so selected is near the left end of the window
			windowStart = 0;
		} else {
			// Keep selected in the middle-ish
			windowStart = selectedIndex - Math.floor(dotCount / 2);
			windowStart = Math.max(0, Math.min(windowStart, totalItems - dotCount));
		}
	}

	// Determine which dots are "edge" dots that should shrink to indicate more items
	const hasMoreLeft = windowStart > 0;
	const hasMoreRight = windowStart + dotCount < totalItems;

	const dotRadius = 2;
	const dotSpacing = 6;
	const totalWidth = dotCount * dotSpacing;
	const startX = (200 - totalWidth) / 2;
	const y = 92;

	for (let i = 0; i < dotCount; i++) {
		const itemIndex = windowStart + i;
		const isSelected = itemIndex === selectedIndex;
		const x = startX + i * dotSpacing + dotSpacing / 2;

		// Shrink edge dots to indicate more items beyond
		let radius = dotRadius;
		if (hasMoreLeft && i === 0) radius = 1;
		if (hasMoreLeft && i === 1 && dotCount > 3) radius = 1.5;
		if (hasMoreRight && i === dotCount - 1) radius = 1;
		if (hasMoreRight && i === dotCount - 2 && dotCount > 3) radius = 1.5;

		ctx.save();
		if (isSelected) {
			// Selected: brighter and wider ellipse
			ctx.fillStyle = "rgba(255, 255, 255, 0.9)";
			ctx.beginPath();
			ctx.ellipse(x, y, radius + 1.5, radius, 0, 0, Math.PI * 2);
			ctx.fill();
		} else {
			ctx.fillStyle = "rgba(255, 255, 255, 0.4)";
			ctx.beginPath();
			ctx.arc(x, y, radius, 0, Math.PI * 2);
			ctx.fill();
		}
		ctx.restore();
	}
}

export async function renderActionWheel(
	canvas: HTMLCanvasElement | null,
	slotContext: Context | null,
	children: ActionInstance[],
	selectedIndex: number,
	active: boolean,
	pressed: boolean,
	fallbackIcon: string,
) {
	if (!canvas) return;

	// Render the three-icon preview at native encoder LCD resolution (200x100) for the device.
	const lcd = document.createElement("canvas");
	lcd.width = 200;
	lcd.height = 100;
	const lctx = lcd.getContext("2d");
	if (!lctx) return;
	const c = lctx;

	c.clearRect(0, 0, lcd.width, lcd.height);
	c.fillStyle = "#000000";
	c.fillRect(0, 0, lcd.width, lcd.height);

	async function drawIcon(ctx: CanvasRenderingContext2D, child: ActionInstance, x: number, y: number, size: number, dimmed: boolean) {
		const state = child.states[child.current_state] ?? child.states[0];
		const fallback = child.action.states[child.current_state]?.image ?? child.action.icon;
		const img = document.createElement("img");
		img.crossOrigin = "anonymous";
		img.src = getImage(state?.image, fallback);
		await new Promise((resolve, reject) => {
			img.onload = resolve;
			img.onerror = reject;
		}).catch(() => {});
		ctx.save();
		if (dimmed) ctx.globalAlpha = 0.33;
		ctx.imageSmoothingQuality = "high";
		ctx.drawImage(img, x, y, size, size);
		ctx.restore();
	}

	if (children.length > 0) {
		const len = children.length;
		const prevIdx = (selectedIndex + len - 1) % len;
		const nextIdx = (selectedIndex + 1) % len;

		const sideSize = 40;
		const centerSize = pressed ? Math.round(56 * 0.8) : 56;
		const gap = 8;
		const totalWidth = sideSize + gap + centerSize + gap + sideSize;
		const startX = (lcd.width - totalWidth) / 2;
		const sideY = (lcd.height - sideSize) / 2;
		const centerY = (lcd.height - centerSize) / 2;

		await drawIcon(c, children[prevIdx], startX, sideY, sideSize, true);
		await drawIcon(c, children[selectedIndex], startX + sideSize + gap, centerY, centerSize, false);
		await drawIcon(c, children[nextIdx], startX + sideSize + gap + centerSize + gap, sideY, sideSize, true);

		// Draw dot indicator at the bottom center
		drawDotIndicator(c, len, selectedIndex);
	}

	// Send the three-icon preview to the device.
	if (active && slotContext) await invoke("update_image", { context: slotContext, image: lcd.toDataURL("image/jpeg") });

	// Render the container's placeholder icon on the visible UI canvas (not the three-icon preview).
	const vctx = canvas.getContext("2d");
	if (vctx) {
		const iconImg = document.createElement("img");
		iconImg.crossOrigin = "anonymous";
		iconImg.src = getImage(fallbackIcon, undefined);
		await new Promise((resolve, reject) => {
			iconImg.onload = resolve;
			iconImg.onerror = reject;
		}).catch(() => {});
		vctx.clearRect(0, 0, canvas.width, canvas.height);
		vctx.imageSmoothingQuality = "high";
		const scaleFactor = pressed ? 0.8 : 1.0;
		const scale = (Math.max(10, 100) / 100) * scaleFactor;
		const xScaled = canvas.width * scale;
		const yScaled = canvas.height * scale;
		const xOffset = (canvas.width - xScaled) / 2;
		const yOffset = (canvas.height - yScaled) / 2;
		vctx.drawImage(iconImg, xOffset, yOffset, xScaled, yScaled);
	}
}

export async function resizeImage(source: string): Promise<string | undefined> {
	const canvas = document.createElement("canvas");
	canvas.width = 288;
	canvas.height = 288;
	const context = canvas.getContext("2d");
	if (!context) return;

	const image = document.createElement("img");
	image.crossOrigin = "anonymous";
	image.src = source;
	await new Promise((resolve) => (image.onload = resolve));

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
