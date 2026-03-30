import type { Action } from "./Action.ts";
import type { Context } from "./Context.ts";

import { type Writable, writable } from "svelte/store";

export const inspectedInstance: Writable<string | Context | null> = writable(null);

import { invoke } from "@tauri-apps/api/core";
let old: string | Context | null = null;
inspectedInstance.subscribe(async (value) => {
	await invoke("switch_property_inspector", {
		old: typeof old == "string" ? old : null,
		new: typeof value == "string" ? value : null,
	});
	old = value;
});

export const inspectedParentAction: Writable<Context | null> = writable(null);

export const openContextMenu: Writable<{ context: Context; x: number; y: number } | null> = writable(null);
document.addEventListener("click", () => openContextMenu.set(null));
document.addEventListener("keydown", (event) => {
	if (event.key == "Escape") openContextMenu.set(null);
});
globalThis.addEventListener("blur", () => openContextMenu.set(null));

export type CopiedItem =
	| { type: "instance"; source: Context }
	| { type: "action"; action: Action };
export const copiedItem: Writable<CopiedItem | null> = writable(null);
