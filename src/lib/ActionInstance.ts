import type { Action } from "./Action.ts";
import type { ActionState } from "./ActionState.ts";

export type ActionInstance = {
	action: Action;
	context: string;
	states: ActionState[];
	current_state: number;
	settings: any;
	children: ActionInstance[] | null;
	/** Layout ID (built-in `$X1` etc.) or path to custom layout JSON. */
	feedback_layout?: string | null;
	/** Accumulated setFeedback state, keyed by layout item key. */
	feedback?: Record<string, unknown> | null;
};
