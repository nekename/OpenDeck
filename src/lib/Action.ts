import type { ActionState } from "./ActionState.ts";

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
	states: ActionState[];
};
