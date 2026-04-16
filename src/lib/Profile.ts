import type { ActionInstance } from "./ActionInstance.ts";

export type Profile = {
	device: string;
	id: string;
	keys: (ActionInstance | null)[];
	sliders: (ActionInstance | null)[];
	swipe_left?: string | null;
	swipe_right?: string | null;
};
