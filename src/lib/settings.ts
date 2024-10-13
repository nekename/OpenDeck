export type Settings = {
	language: string;
	background: boolean;
	autolaunch: boolean;
	darktheme: boolean;
	brightness: number;
	developer: boolean;
};

import { invoke } from "@tauri-apps/api/core";
import { type Writable, writable } from "svelte/store";

export const settings: Writable<Settings | null> = writable(null);
(async () => settings.set(await invoke("get_settings")))();
export const localisations: Writable<{ [plugin: string]: any } | null> = writable(null);
settings.subscribe(async (value) => {
	if (value) {
		await invoke("set_settings", { settings: value });
		localisations.set(await invoke("get_localisations", { locale: value.language }));
	}
});
