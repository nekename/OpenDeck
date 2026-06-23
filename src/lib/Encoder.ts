export type Encoder = {
	icon: string;
	stack_color: string;
	trigger_description: TriggerDescription;
	background: string;
	layout: string;
	layout_parsed: object;
};

export type TriggerDescription = {
	long_touch: string;
	push: string;
	rotate: string;
	touch: string;
};
