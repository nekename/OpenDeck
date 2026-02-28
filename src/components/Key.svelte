<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { ActionState } from "$lib/ActionState";
	import type { Context } from "$lib/Context";

	import Clipboard from "phosphor-svelte/lib/Clipboard";
	import Copy from "phosphor-svelte/lib/Copy";
	import Pencil from "phosphor-svelte/lib/Pencil";
	import Trash from "phosphor-svelte/lib/Trash";
	import InstanceEditor from "./InstanceEditor.svelte";

	import { copiedContext, inspectedInstance, inspectedParentAction, openContextMenu } from "$lib/propertyInspector";
	import { CanvasLock, renderImage } from "$lib/rendererHelper";
	import { settings } from "$lib/settings";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";

	export let context: Context | null;
	export let label: string = "";

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
	export let isTouchPoint: boolean = false;
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

	function select(event: MouseEvent | KeyboardEvent) {
		if (event instanceof MouseEvent && event.ctrlKey) return;
		$openContextMenu = null;
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
		$openContextMenu = { context, x: event.x, y: event.y };
	}

	let showEditor = false;
	function edit() {
		showEditor = true;
	}

	export let handlePaste: ((source: Context, destination: Context) => void) | undefined = undefined;
	async function paste() {
		if (!$copiedContext || !context) return;
		if (handlePaste) handlePaste($copiedContext, context);
	}

	async function clear() {
		if (!slot) return;
		await invoke("remove_instance", { context: slot.context });
		if ($inspectedInstance == slot.context) inspectedInstance.set(null);
		showEditor = false;
		slot = null;
		inslot = slot;
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

	$: accessibleLabel = label + (slot ? ": " + slot.action.name + (state?.show && state?.text ? " - " + state.text : "") : "");
	$: (async () => {
		const sl = structuredClone(slot);
		if (!sl) {
			const unlock = await lock.lock();
			try {
				const ctx = canvas?.getContext("2d");
				if (ctx) ctx.clearRect(0, 0, canvas.width, canvas.height);
				if (active) await invoke("update_image", { context, image: null });
			} finally {
				unlock();
			}
		} else {
			const unlock = await lock.lock();
			try {
				let fallback = sl.action.states[sl.current_state]?.image ?? sl.action.icon;
				if (state) await renderImage(canvas, context, state, fallback, showOk, showAlert, true, active, pressed, $settings?.rotation);
			} finally {
				unlock();
			}
		}
	})();

	function clearAndRedraw() {
		canvas?.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height);
		slot = slot;
	}
	$: if ($settings?.rotation != undefined) {
		clearAndRedraw();
	}
</script>

<div
	class="relative"
	style={`transform: scale(${(112 /* desired inner size */ / size) * scale});`}
>
	<canvas
		bind:this={canvas}
		class="relative border-3 border-neutral-700 rounded-3xl outline-none outline-offset-2 outline-blue-500"
		style={`margin: ${-((size + 3 * 2 /* border */ - 132 /* desired outer size */) / 2)}px;`}
		class:outline-solid={slot && $inspectedInstance == slot.context}
		class:rounded-full!={context?.controller == "Encoder"}
		class:bg-black={slot != null}
		width={size}
		height={size}
		draggable={slot != null}
		tabindex="0"
		role="button"
		aria-label={accessibleLabel}
		on:dragstart
		on:dragover
		on:drop
		on:click|stopPropagation={select}
		on:keyup|stopPropagation={(e) => { if (e.key === "Enter" || e.key === " ") select(e); }}
		on:contextmenu={contextMenu}
	/>
	{#if isTouchPoint && !slot}
		<div class="absolute left-1/4 top-1/2 w-1/2 border-t-4 border-neutral-700 pointer-events-none"></div>
	{/if}
</div>

{#if $openContextMenu && $openContextMenu?.context == context}
	<div
		class="absolute w-32 font-semibold text-sm text-neutral-300 bg-neutral-700 border border-neutral-600 rounded-lg divide-y divide-neutral-600! z-10"
		style={`left: ${$openContextMenu.x}px; top: ${$openContextMenu.y}px;`}
	>
		{#if !slot}
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-lg cursor-pointer"
				on:click={paste}
			>
				<Clipboard size="18" class="text-neutral-300" />
				<span class="ml-2"> Paste </span>
			</button>
		{:else}
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-t-lg cursor-pointer"
				on:click={edit}
			>
				<Pencil size="18" class="text-neutral-300" />
				<span class="ml-2"> Edit </span>
			</button>
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors cursor-pointer"
				on:click={() => copiedContext.set(context)}
			>
				<Copy size="18" class="text-neutral-300" />
				<span class="ml-2"> Copy </span>
			</button>
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-b-lg cursor-pointer"
				on:click={clear}
			>
				<Trash size="18" class="text-red-400" />
				<span class="ml-2"> Delete </span>
			</button>
		{/if}
	</div>
{/if}

{#if slot && showEditor}
	<InstanceEditor bind:instance={slot} bind:showEditor />
{/if}
