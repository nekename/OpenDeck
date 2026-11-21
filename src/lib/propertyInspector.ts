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

export type CopiedItem = { type: "instance"; source: Context } | { type: "action"; action: Action };
export const copiedItem: Writable<CopiedItem | null> = writable(null);

/**
 * Copies an item into this window and into the Rust side, so the other window
 * can paste it too. The Rust side emits "clipboard", which keeps both windows
 * in sync without them talking to each other directly.
 */
export async function copyItem(item: CopiedItem) {
	copiedItem.set(item);
	try {
		await invoke("set_clipboard", { item: JSON.stringify(item) });
	} catch (error) {
		console.error("Could not share the copied item", error);
	}
}

/** Loads a clipboard that was copied in another window. */
export async function initClipboard() {
	try {
		const raw = await invoke<string | null>("get_clipboard");
		if (raw) copiedItem.set(JSON.parse(raw));
	} catch (error) {
		console.error("Could not read the shared clipboard", error);
	}

	await listen<string | null>("clipboard", ({ payload }) => {
		copiedItem.set(payload ? JSON.parse(payload) : null);
	});
}
