import { writable } from "svelte/store";

/** The background image of each device. */
export const deviceBackgrounds = writable<{ [id: string]: string | null }>({});
