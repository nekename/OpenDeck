import type { Context } from "./Context.ts";

import { type Writable, writable } from "svelte/store";

export const inspectedInstance: Writable<string | null> = writable(null);

import { invoke } from "@tauri-apps/api/core";
let old: string | null = null;
inspectedInstance.subscribe(async (value) => {
	await invoke("switch_property_inspector", { old, new: value });
	old = value;
});

export const inspectedParentAction: Writable<Context | null> = writable(null);

export const openContextMenu: Writable<{ context: Context; x: number; y: number } | null> = writable(null);
document.addEventListener("click", () => openContextMenu.set(null));
globalThis.addEventListener("blur", () => openContextMenu.set(null));

export const copiedContext: Writable<Context | null> = writable(null);
