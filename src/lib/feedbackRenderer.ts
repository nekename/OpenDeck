// Renders a Stream Deck + feedback layout (built-in or custom) onto a 200x100
// canvas by composing the layout items with the current feedback state.
//
// Spec references used throughout:
//   https://docs.elgato.com/streamdeck/sdk/references/touch-strip-layout/
//   https://docs.elgato.com/streamdeck/sdk/guides/dials/

import type { BarItem, GBarItem, Layout, LayoutItem, LayoutRect, PixmapItem, TextItem } from "./feedbackLayouts.ts";

import { getImage } from "./rendererHelper.ts";

export const CANVAS_WIDTH = 200;
export const CANVAS_HEIGHT = 100;

type FeedbackState = Record<string, unknown>;

// Merge a setFeedback payload value into an item. Payload values can be
// scalar (shorthand for `.value`) or an object with property overrides.
function mergeItem<T extends LayoutItem>(item: T, patch: unknown): T {
	if (patch == null) return item;
	if (typeof patch === "object" && !Array.isArray(patch)) {
		return { ...item, ...(patch as Partial<T>) };
	}
	// Scalar: treat as the item's primary value.
	return { ...item, value: patch as T["value"] };
}

// Apply feedback state to a layout, returning items ready to render. Items
// are sorted by zOrder so overlays draw on top of backgrounds.
export function applyFeedback(layout: Layout, state: FeedbackState | undefined): LayoutItem[] {
	const resolved = layout.items.map((item) => {
		const patch = state ? (state as Record<string, unknown>)[item.key] : undefined;
		return mergeItem(item, patch);
	});
	resolved.sort((a, b) => (a.zOrder ?? 0) - (b.zOrder ?? 0));
	return resolved;
}

// Parse a colour spec into something canvas can consume. Supports named
// colours, hex, and gradient strings like "0:#ff0000,0.5:yellow,1:#00ff00".
function resolveColour(ctx: CanvasRenderingContext2D, spec: string | undefined, rect: LayoutRect, fallback: string): string | CanvasGradient {
	if (!spec) return fallback;
	if (!spec.includes(":") || !spec.includes(",")) return spec;

	// Gradient format: "{offset}:{colour}[,{offset}:{colour}...]"
	const stops = spec.split(",").map((s) => {
		const [offset, colour] = s.split(":");
		return { offset: parseFloat(offset), colour: colour.trim() };
	}).filter((s) => !isNaN(s.offset) && s.colour);
	if (stops.length < 2) return fallback;

	const [x, y, w, h] = rect;
	const gradient = ctx.createLinearGradient(x, y, x + w, y);
	for (const stop of stops) gradient.addColorStop(Math.max(0, Math.min(1, stop.offset)), stop.colour);
	return gradient;
}

async function loadImage(src: string): Promise<HTMLImageElement | null> {
	const img = document.createElement("img");
	img.crossOrigin = "anonymous";
	img.src = getImage(src, undefined);
	try {
		await new Promise((resolve, reject) => {
			img.onload = resolve;
			img.onerror = reject;
		});
		return img;
	} catch {
		return null;
	}
}

async function drawPixmap(ctx: CanvasRenderingContext2D, item: PixmapItem) {
	if (item.enabled === false || !item.value) return;
	const [x, y, w, h] = item.rect;
	ctx.save();
	if (item.opacity != null) ctx.globalAlpha = item.opacity;
	if (item.background) {
		ctx.fillStyle = resolveColour(ctx, item.background, item.rect, "transparent");
		ctx.fillRect(x, y, w, h);
	}
	const image = await loadImage(item.value);
	if (image) {
		// Preserve aspect ratio inside the item rect -- do not stretch to rect
		// unless the key is `full-canvas`, which is explicitly documented as a
		// 200x100 pixmap (i.e. the plugin handed us the whole strip).
		if (item.key === "full-canvas") {
			ctx.drawImage(image, x, y, w, h);
		} else {
			const scale = Math.min(w / image.width, h / image.height);
			const dw = image.width * scale;
			const dh = image.height * scale;
			const dx = x + (w - dw) / 2;
			const dy = y + (h - dh) / 2;
			ctx.drawImage(image, dx, dy, dw, dh);
		}
	}
	ctx.restore();
}

function drawText(ctx: CanvasRenderingContext2D, item: TextItem) {
	if (item.enabled === false) return;
	const value = item.value;
	if (value == null || value === "") return;
	const [x, y, w, h] = item.rect;

	ctx.save();
	if (item.opacity != null) ctx.globalAlpha = item.opacity;
	if (item.background) {
		ctx.fillStyle = resolveColour(ctx, item.background, item.rect, "transparent");
		ctx.fillRect(x, y, w, h);
	}

	const size = item.font?.size ?? 16;
	const weight = item.font?.weight ?? 400;
	ctx.font = `${weight} ${size}px "SF Pro", "Segoe UI", "Helvetica Neue", system-ui, sans-serif`;
	ctx.fillStyle = item.color ?? "white";
	ctx.textBaseline = "middle";

	let text = String(value);
	// Handle overflow in the most minimal way that matches the spec.
	const overflow = item["text-overflow"] ?? "clip";
	if (overflow === "ellipsis") {
		const ellipsis = "…";
		if (ctx.measureText(text).width > w) {
			while (text.length > 0 && ctx.measureText(text + ellipsis).width > w) text = text.slice(0, -1);
			text = text + ellipsis;
		}
	} else if (overflow === "clip") {
		// `clip` truncates at the boundary; canvas `fillText` does this implicitly
		// when we apply a clip rect.
		ctx.beginPath();
		ctx.rect(x, y, w, h);
		ctx.clip();
	} // `fade` would need a mask gradient; skipping for now to avoid faking
	  // spec behaviour. Text will still render clipped to the item rect.

	const align = item.alignment ?? "center";
	ctx.textAlign = align;
	const textX = align === "left" ? x : align === "right" ? x + w : x + w / 2;
	const textY = y + h / 2;
	ctx.fillText(text, textX, textY);
	ctx.restore();
}

// Map a value+range pair to a fraction in [0, 1].
function valueFraction(value: number, range: { min: number; max: number } | undefined): number {
	const min = range?.min ?? 0;
	const max = range?.max ?? 100;
	if (max <= min) return 0;
	return Math.max(0, Math.min(1, (value - min) / (max - min)));
}

// Bar subtypes per spec: 0 Rectangle, 1 DoubleRectangle, 2 Trapezoid, 3
// DoubleTrapezoid, 4 Groove (default).
function drawBarShape(ctx: CanvasRenderingContext2D, rect: LayoutRect, subtype: number) {
	const [x, y, w, h] = rect;
	ctx.beginPath();
	switch (subtype) {
		case 2: // Trapezoid: tall on the right
			ctx.moveTo(x, y + h);
			ctx.lineTo(x + w, y);
			ctx.lineTo(x + w, y + h);
			ctx.closePath();
			break;
		case 3: // DoubleTrapezoid: tall in the centre
			ctx.moveTo(x, y + h);
			ctx.lineTo(x + w / 2, y);
			ctx.lineTo(x + w, y + h);
			ctx.closePath();
			break;
		case 4: // Groove: rounded bar
			{
				const r = h / 2;
				ctx.moveTo(x + r, y);
				ctx.arcTo(x + w, y, x + w, y + h, r);
				ctx.arcTo(x + w, y + h, x, y + h, r);
				ctx.arcTo(x, y + h, x, y, r);
				ctx.arcTo(x, y, x + w, y, r);
				ctx.closePath();
			}
			break;
		case 1: // DoubleRectangle: two stacked rectangles
			ctx.rect(x, y, w, h / 2 - 1);
			ctx.rect(x, y + h / 2 + 1, w, h / 2 - 1);
			break;
		case 0: // Rectangle
		default:
			ctx.rect(x, y, w, h);
			break;
	}
}

function drawBar(ctx: CanvasRenderingContext2D, item: BarItem) {
	if (item.enabled === false) return;
	const [x, y, w, h] = item.rect;
	ctx.save();
	if (item.opacity != null) ctx.globalAlpha = item.opacity;
	if (item.background) {
		ctx.fillStyle = resolveColour(ctx, item.background, item.rect, "transparent");
		ctx.fillRect(x, y, w, h);
	}

	const subtype = item.subtype ?? 4;
	const borderWidth = item.border_w ?? 2;
	const fraction = valueFraction(item.value, item.range);

	// Background of the bar itself
	drawBarShape(ctx, item.rect, subtype);
	ctx.fillStyle = resolveColour(ctx, item.bar_bg_c, item.rect, "#222");
	ctx.fill();

	// Fill portion, clipped to the bar shape and to `fraction * width`.
	if (fraction > 0) {
		ctx.save();
		drawBarShape(ctx, item.rect, subtype);
		ctx.clip();
		ctx.fillStyle = resolveColour(ctx, item.bar_fill_c, item.rect, "white");
		ctx.fillRect(x, y, w * fraction, h);
		ctx.restore();
	}

	// Border
	if (borderWidth > 0) {
		drawBarShape(ctx, item.rect, subtype);
		ctx.strokeStyle = item.bar_border_c ?? "white";
		ctx.lineWidth = borderWidth;
		ctx.stroke();
	}
	ctx.restore();
}

function drawGBar(ctx: CanvasRenderingContext2D, item: GBarItem) {
	if (item.enabled === false) return;
	const [x, y, w, h] = item.rect;
	ctx.save();
	if (item.opacity != null) ctx.globalAlpha = item.opacity;
	if (item.background) {
		ctx.fillStyle = resolveColour(ctx, item.background, item.rect, "transparent");
		ctx.fillRect(x, y, w, h);
	}

	// gbar renders a shorter bar inside the item rect, with a triangle
	// indicator pointing up from below. `bar_h` is the bar's own height.
	const barH = item.bar_h ?? Math.max(4, h - 8);
	const barRect: LayoutRect = [x, y, w, barH];
	const subtype = item.subtype ?? 4;

	drawBarShape(ctx, barRect, subtype);
	ctx.fillStyle = resolveColour(ctx, item.bar_bg_c, barRect, "#222");
	ctx.fill();

	if ((item.border_w ?? 0) > 0) {
		drawBarShape(ctx, barRect, subtype);
		ctx.strokeStyle = item.bar_border_c ?? "white";
		ctx.lineWidth = item.border_w!;
		ctx.stroke();
	}

	// Triangle indicator beneath the bar at `fraction * width`.
	const fraction = valueFraction(item.value, item.range);
	const indicatorX = x + w * fraction;
	const indicatorTop = y + barH + 1;
	const indicatorHalf = Math.max(3, (h - barH) / 2);

	ctx.beginPath();
	ctx.moveTo(indicatorX, indicatorTop);
	ctx.lineTo(indicatorX - indicatorHalf, indicatorTop + indicatorHalf * 2);
	ctx.lineTo(indicatorX + indicatorHalf, indicatorTop + indicatorHalf * 2);
	ctx.closePath();
	ctx.fillStyle = item.bar_fill_c ? String(resolveColour(ctx, item.bar_fill_c, item.rect, "white")) : "white";
	ctx.fill();

	ctx.restore();
}

export async function renderFeedback(
	canvas: HTMLCanvasElement,
	layout: Layout,
	state: FeedbackState | undefined,
): Promise<void> {
	canvas.width = CANVAS_WIDTH;
	canvas.height = CANVAS_HEIGHT;
	const ctx = canvas.getContext("2d");
	if (!ctx) return;

	// Black background matches the device's default idle state.
	ctx.fillStyle = "#000";
	ctx.fillRect(0, 0, canvas.width, canvas.height);

	const items = applyFeedback(layout, state);

	// Pixmaps must be awaited; other item types are synchronous. Walk items in
	// z-order and resolve sequentially so later items draw on top.
	for (const item of items) {
		switch (item.type) {
			case "pixmap":
				await drawPixmap(ctx, item);
				break;
			case "text":
				drawText(ctx, item);
				break;
			case "bar":
				drawBar(ctx, item);
				break;
			case "gbar":
				drawGBar(ctx, item);
				break;
		}
	}
}
