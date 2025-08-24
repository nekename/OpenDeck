<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { ActionState } from "$lib/ActionState";
	import type { Context } from "$lib/Context";

	import InstanceEditor from "./InstanceEditor.svelte";
	import ContextMenu from "./ContextMenu.svelte";

	import { copiedContext, inspectedInstance, inspectedParentAction, openContextMenu } from "$lib/propertyInspector";
	import { CanvasLock, renderImage } from "$lib/rendererHelper";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";

	export let context: Context | null;

	// One-way binding for slot data.
	export let inslot: ActionInstance | null;
	let slot: ActionInstance | null;
	const update = (inslot: ActionInstance | null) => {
		if (inslot && context && inslot.context.split(".")[0] != context.device) return;
		slot = inslot;
	};
	$: update(inslot);

	export let active: boolean = true;
	export let scale: number = 1;
	let pressed: boolean = false;

	let state: ActionState | undefined;
	$: {
		if (!slot) {
			state = undefined;
		} else {
			state = slot.states[slot.current_state];
		}
	}

	listen("update_state", ({ payload }: { payload: { context: string; contents: ActionInstance | null } }) => {
		if (payload.context == slot?.context) slot = payload.contents;
	});

	listen("key_moved", ({ payload }: { payload: { context: Context; pressed: boolean } }) => {
		if (JSON.stringify(context) == JSON.stringify(payload.context)) pressed = payload.pressed;
	});

	function select() {
		if (!slot) return;
		if (slot.action.uuid == "opendeck.multiaction" || slot.action.uuid == "opendeck.toggleaction") {
			inspectedParentAction.set(context);
		} else {
			inspectedInstance.set(slot.context);
		}
	}

	async function contextMenu(event: MouseEvent) {
		event.preventDefault();
		if (!active || !context) return;
		if (event.ctrlKey) return;
		$openContextMenu = { context, x: event.x, y: event.y };
	}

	let showEditor = false;
	function edit() {
		showEditor = true;
		openContextMenu.set(null);
	}

	export let handlePaste: ((source: Context, destination: Context) => void) | undefined = undefined;
	async function paste() {
		if (!$copiedContext || !context) return;
		if (handlePaste) handlePaste($copiedContext, context);
		openContextMenu.set(null);
		announceToScreenReader("Action pasted.");
	}

	async function clear() {
		if (!slot) return;
		await invoke("remove_instance", { context: slot.context });
		if ($inspectedInstance == slot.context) inspectedInstance.set(null);
		showEditor = false;
		slot = null;
		inslot = slot;
		openContextMenu.set(null);
		announceToScreenReader("Action removed from key.");
	}

	let showAlert: boolean = false;
	let showOk: boolean = false;
	let timeouts: number[] = [];
	listen("show_alert", ({ payload }: { payload: string }) => {
		if (!slot || payload != slot.context) return;
		timeouts.forEach(clearTimeout);
		showOk = false;
		showAlert = true;
		timeouts.push(setTimeout(() => showAlert = false, 1.5e3));
	});
	listen("show_ok", ({ payload }: { payload: string }) => {
		if (!slot || payload != slot.context) return;
		timeouts.forEach(clearTimeout);
		showAlert = false;
		showOk = true;
		timeouts.push(setTimeout(() => showOk = false, 1.5e3));
	});

	let canvas: HTMLCanvasElement;
	let lock = new CanvasLock();
	export let size = 144;
	$: (async () => {
		const sl = structuredClone(slot);
		if (!sl) {
			if (canvas) {
				let context = canvas.getContext("2d");
				if (context) context.clearRect(0, 0, canvas.width, canvas.height);
			}
		} else {
			const unlock = await lock.lock();
			try {
				let fallback = sl.action.states[sl.current_state]?.image ?? sl.action.icon;
				if (state) await renderImage(canvas, context, state, fallback, showOk, showAlert, true, active, pressed);
			} finally {
				unlock();
			}
		}
	})();

	// Accessibility: keyboard navigation and action assignment
	export let focused: boolean = false;
	let announcementEl: HTMLElement;

	function announceToScreenReader(message: string) {
		if (announcementEl) {
			announcementEl.textContent = message;
		}
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (!context) return;

		// Don't handle keyboard events if context menu is open
		if ($openContextMenu && $openContextMenu.context === context) {
			return;
		}

		// Allow arrow keys to bubble up for grid navigation
		if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(event.key)) {
			return;
		}

		switch (event.key) {
			case "Enter":
			case " ":
				event.preventDefault();
				event.stopPropagation();
				if (!slot) {
					// Key is empty - try to assign selected action
					const requestEvent = new CustomEvent("requestSelectedAction", {
						detail: { context },
						bubbles: true,
					});
					dispatchEvent(requestEvent);
				} else {
					// If key has action, select it for property inspection
					select();
				}
				break;
			case "Delete":
			case "Backspace":
				event.preventDefault();
				event.stopPropagation();
				if (slot) {
					clear();
				}
				break;
			case "c":
				if (event.ctrlKey && slot) {
					event.preventDefault();
					event.stopPropagation();
					copiedContext.set(context);
					announceToScreenReader("Action copied to clipboard.");
				}
				break;
			case "v":
				if (event.ctrlKey) {
					event.preventDefault();
					event.stopPropagation();
					paste();
				}
				break;
		}
	}

	function getAccessibleLabel(): string {
		if (!context) return "Key";

		const position = context.position + 1;
		const controllerType = context.controller === "Encoder" ? "Encoder" : "Key";

		if (slot) {
			const actionName = slot.action?.name || "Unknown action";
			return `${controllerType} ${position}: ${actionName}`;
		} else {
			return `${controllerType} ${position}: Empty`;
		}
	}

	function getSafeId(): string {
		if (!context) return "key-unknown";
		// Replace problematic characters in IDs to avoid NVDA issues
		// NVDA has issues with dots, colons, and other special characters in IDs
		const safeDevice = (context.device || 'unknown').replace(/[^a-zA-Z0-9]/g, '_');
		const safeProfile = (context.profile || 'default').replace(/[^a-zA-Z0-9]/g, '_');
		const safeController = context.controller || 'Keypad';
		return `key_${safeDevice}_${safeProfile}_${safeController}_${context.position}`;
	}
</script>

<!-- Screen reader announcements -->
<div bind:this={announcementEl} class="sr-only" aria-live="polite" aria-atomic="true"></div>

<canvas
	bind:this={canvas}
	id={getSafeId()}
	class="relative -m-2 border-2 dark:border-neutral-700 rounded-md outline-none outline-offset-2 outline-blue-500"
	class:outline-solid={slot && $inspectedInstance == slot.context}
	class:ring-2={focused}
	class:ring-blue-500={focused}
	class:ring-offset-2={focused}
	class:-m-[2.06rem]={size == 192}
	class:rounded-full!={context?.controller == "Encoder"}
	width={size}
	height={size}
	style={`transform: scale(${(112 / size) * scale});`}
	tabindex="-1"
	role="gridcell"
	aria-label={getAccessibleLabel()}
	aria-describedby={slot ? `${getSafeId()}-desc` : undefined}
	draggable={slot != null}
	on:dragstart
	on:dragover
	on:drop
	on:click|stopPropagation={select}
	on:keydown={handleKeyDown}
	on:focus
	on:blur
	on:contextmenu={contextMenu}
/>

{#if slot}
	<div id="{getSafeId()}-desc" class="sr-only">
		{slot.action?.tooltip || "No description available"}
	</div>
{/if}

{#if $openContextMenu && $openContextMenu?.context == context}
	<ContextMenu
		{slot}
		{context}
		x={$openContextMenu.x}
		y={$openContextMenu.y}
		on:edit={edit}
		on:paste={paste}
		on:clear={clear}
		on:copy={() => {
			copiedContext.set(context);
			openContextMenu.set(null);
			announceToScreenReader("Action copied to clipboard.");
		}}
		on:close={() => openContextMenu.set(null)}
	/>
{/if}

{#if slot && showEditor}
	<InstanceEditor bind:instance={slot} bind:showEditor />
{/if}

<style>
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		border: 0;
	}
</style>
