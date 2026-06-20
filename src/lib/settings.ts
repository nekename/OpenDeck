export type Settings = {
	version: string;
	language: string;
	brightness: number;
	sleep_timeout_minutes: number;
	rotation: number;
	background: boolean;
	autolaunch: boolean;
	updatecheck: boolean;
	statistics: boolean;
	separatewine: boolean;
	developer: boolean;
	disableelgato: boolean;
};

import { invoke } from "@tauri-apps/api/core";
import { type Writable, writable } from "svelte/store";
import { locale } from "./i18n.ts";

export const settings: Writable<Settings | null> = writable(null);
(async () => settings.set(await invoke("get_settings")))();
export const localisations: Writable<{ [plugin: string]: any } | null> = writable(null);
settings.subscribe(async (value) => {
	if (value) {
		locale.set(value.language);
		await invoke("set_settings", { settings: value });
		localisations.set(await invoke("get_localisations", { locale: value.language }));
	}
});
