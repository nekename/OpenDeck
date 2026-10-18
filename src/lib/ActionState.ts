export type ActionState = {
	image: string;
	image_override: string | null;
	image_scale: number;
	background_colour: string;
	name: string;
	text: string;
	text_override: string | null;
	show: boolean;
	colour: string;
	stroke_colour: string;
	alignment: "top" | "middle" | "bottom";
	family: string;
	style: string;
	size: number;
	stroke_size: number;
	underline: boolean;
};
