import type { ActionState } from "./ActionState.ts";
import type { Encoder } from "./Encoder.ts";

export type Action = {
	name: string;
	uuid: string;
	plugin: string;
	tooltip: string;
	icon: string;
	visible_in_action_list: boolean;
	supported_in_multi_actions: boolean;
	property_inspector: string;
	controllers: string[];
	encoder?: Encoder | null;
	states: ActionState[];
};
