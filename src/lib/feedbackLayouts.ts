// Stream Deck + touch strip layouts.
// Spec: https://docs.elgato.com/streamdeck/sdk/references/touch-strip-layout/
// Built-in layout JSON copied verbatim from:
//   https://docs.elgato.com/streamdeck/sdk/guides/dials/#built-in-layouts

export type LayoutRect = [x: number, y: number, width: number, height: number];

export type LayoutItemBase = {
	key: string;
	rect: LayoutRect;
	background?: string;
	enabled?: boolean;
	opacity?: number;
	zOrder?: number;
};

export type BarItem = LayoutItemBase & {
	type: "bar";
	value: number;
	range?: { min: number; max: number };
	subtype?: 0 | 1 | 2 | 3 | 4;
	bar_bg_c?: string;
	bar_fill_c?: string;
	bar_border_c?: string;
	border_w?: number;
};

export type GBarItem = LayoutItemBase & {
	type: "gbar";
	value: number;
	range?: { min: number; max: number };
	subtype?: 0 | 1 | 2 | 3 | 4;
	bar_h?: number;
	bar_bg_c?: string;
	bar_fill_c?: string;
	bar_border_c?: string;
	border_w?: number;
};

export type PixmapItem = LayoutItemBase & {
	type: "pixmap";
	value?: string;
};

export type TextItem = LayoutItemBase & {
	type: "text";
	value?: string;
	alignment?: "center" | "left" | "right";
	color?: string;
	font?: { size?: number; weight?: number };
	"text-overflow"?: "clip" | "ellipsis" | "fade";
};

export type LayoutItem = BarItem | GBarItem | PixmapItem | TextItem;

export type Layout = {
	id: string;
	items: LayoutItem[];
};

// Default: centered icon with a left-aligned title above.
const $X1: Layout = {
	id: "$X1",
	items: [
		{ key: "title", type: "text", rect: [16, 10, 136, 24], font: { size: 16, weight: 600 }, alignment: "left" },
		{ key: "icon", type: "pixmap", rect: [76, 40, 48, 48] },
	],
};

// Full 200x100 custom canvas (plus title overlay and inner canvas slot).
const $A0: Layout = {
	id: "$A0",
	items: [
		{ key: "full-canvas", type: "pixmap", rect: [0, 0, 200, 100] },
		{ key: "title", type: "text", rect: [16, 10, 136, 24], zOrder: 1, font: { size: 16, weight: 600 }, alignment: "left" },
		{ key: "canvas", type: "pixmap", rect: [16, 34, 136, 54], zOrder: 1 },
	],
};

// Title + icon + large value text.
const $A1: Layout = {
	id: "$A1",
	items: [
		{ key: "title", type: "text", rect: [16, 10, 136, 24], font: { size: 16, weight: 600 }, alignment: "left" },
		{ key: "icon", type: "pixmap", rect: [16, 40, 48, 48] },
		{ key: "value", type: "text", rect: [76, 40, 108, 32], font: { size: 24, weight: 600 }, alignment: "right" },
	],
};

// Title + icon + value + horizontal groove bar.
const $B1: Layout = {
	id: "$B1",
	items: [
		{ key: "title", type: "text", rect: [16, 10, 136, 24], font: { size: 16, weight: 600 }, alignment: "left" },
		{ key: "icon", type: "pixmap", rect: [16, 40, 48, 48] },
		{ key: "value", type: "text", rect: [76, 40, 108, 32], font: { size: 24, weight: 600 }, alignment: "right" },
		{ key: "indicator", type: "bar", rect: [76, 74, 108, 12], value: 0, subtype: 4, border_w: 0 },
	],
};

// Same as $B1 but the bar has a gradient fill and a triangle indicator.
const $B2: Layout = {
	id: "$B2",
	items: [
		{ key: "title", type: "text", rect: [16, 10, 136, 24], font: { size: 16, weight: 600 }, alignment: "left" },
		{ key: "icon", type: "pixmap", rect: [16, 40, 48, 48] },
		{ key: "value", type: "text", rect: [76, 40, 108, 32], font: { size: 24, weight: 600 }, alignment: "right" },
		{
			key: "indicator",
			type: "gbar",
			rect: [76, 74, 108, 20],
			value: 0,
			subtype: 4,
			bar_h: 12,
			border_w: 0,
			bar_bg_c: "0:#ff0000,0.33:#a6d4ec,0.66:#f4b675,1:#00ff00",
		},
	],
};

// Title + two stacked icon+bar rows.
const $C1: Layout = {
	id: "$C1",
	items: [
		{ key: "title", type: "text", rect: [16, 10, 136, 24], font: { size: 16, weight: 600 }, alignment: "left" },
		{ key: "icon1", type: "pixmap", rect: [16, 40, 24, 24] },
		{ key: "icon2", type: "pixmap", rect: [16, 68, 24, 24] },
		{ key: "indicator1", type: "bar", rect: [48, 46, 136, 12], value: 0, subtype: 4, border_w: 0 },
		{ key: "indicator2", type: "bar", rect: [48, 74, 136, 12], value: 0, subtype: 4, border_w: 0 },
	],
};

export const BUILT_IN_LAYOUTS: Record<string, Layout> = {
	$X1, $A0, $A1, $B1, $B2, $C1,
};

export function getBuiltIn(id: string): Layout | undefined {
	return BUILT_IN_LAYOUTS[id];
}
