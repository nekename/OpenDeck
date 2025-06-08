import type { Action } from "./Action.ts";
import type { ActionState } from "./ActionState.ts";

export type ActionInstance = {
	action: Action;
	context: string;
	states: ActionState[];
	current_state: number;
	settings: any;
	children: ActionInstance[] | null;
};
