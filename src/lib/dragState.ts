import { writable } from "svelte/store";

// Shared drag state so drop targets can highlight based on what's being
// dragged and which slot the cursor is currently over.

export type DragInfo = {
	controllers: string[];
} | null;

export const dragAction = writable<DragInfo>(null);
export const hoveredSlot = writable<{ controller: string; position: number } | null>(null);

// Only one encoder dial's virtual controls panel should be open at a time.
export const openDialPosition = writable<number | null>(null);
