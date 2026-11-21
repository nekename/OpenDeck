import type { ActionInstance } from "./ActionInstance.ts";

export type ProfilePage = {
	keys: (ActionInstance | null)[];
	sliders: (ActionInstance | null)[];
	infobars: (ActionInstance | null)[];
};

export type Profile = {
	device: string;
	id: string;
	pages: ProfilePage[];
	current_page: number;
	keys: (ActionInstance | null)[];
	sliders: (ActionInstance | null)[];
	infobars: (ActionInstance | null)[];
};
