<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { ActionState } from "$lib/ActionState";
	import type { Context } from "$lib/Context";
	import type { CopiedItem } from "$lib/propertyInspector";

	import Clipboard from "phosphor-svelte/lib/Clipboard";
	import Copy from "phosphor-svelte/lib/Copy";
	import Pencil from "phosphor-svelte/lib/Pencil";
	import Trash from "phosphor-svelte/lib/Trash";
	import InstanceEditor from "./InstanceEditor.svelte";

	import { copiedItem, inspectedInstance, inspectedParentAction, openContextMenu } from "$lib/propertyInspector";
	import { CanvasLock, renderImage } from "$lib/rendererHelper";
	import { getBuiltIn, type Layout } from "$lib/feedbackLayouts";
	import { renderFeedback } from "$lib/feedbackRenderer";
	import { settings } from "$lib/settings";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { tick } from "svelte";

	export let context: Context | null;
	export let label: string = "";
	export let tabindex: number = 0;
	export let role: string = "gridcell";

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
	export let encoderStrip: boolean = false;
	export let dragHighlight: "empty" | "occupied" | "hovered" | "incompatible" | null = null;
	export let encoderPosition: number = 0;
	export let encoderCount: number = 4;
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

	// Plugin updates to setFeedback / setFeedbackLayout are broadcast from the
	// backend so any encoder slot listening for its own context can re-render
	// the layout and push the resulting pixmap to the device.
	type FeedbackEvent = { context: string; plugin: string; layout: string | null; feedback: Record<string, unknown> | null };
	listen("feedback_changed", async ({ payload }: { payload: FeedbackEvent }) => {
		if (!slot || payload.context !== slot.context) return;
		slot = { ...slot, feedback_layout: payload.layout ?? slot.feedback_layout, feedback: payload.feedback ?? undefined };
	});

	// Resolve the current layout definition. Built-ins are defined client-side;
	// custom layouts are loaded from the plugin folder via a Tauri command.
	let resolvedLayout: Layout | null = null;
	let resolvedLayoutId: string | null = null;
	async function resolveLayout(plugin: string, layoutId: string | null | undefined): Promise<Layout | null> {
		if (!layoutId) return null;
		if (layoutId.startsWith("$")) return getBuiltIn(layoutId) ?? null;
		try {
			const raw = await invoke<Layout>("get_feedback_layout", { plugin, layout: layoutId });
			return raw ?? null;
		} catch (err) {
			console.warn(`[OpenDeck] failed to load custom layout ${layoutId} for ${plugin}:`, err);
			return null;
		}
	}
	// Per the Elgato SDK, encoder actions render feedback via a layout.
	// When the slot has no layout set (plugin manifest omitted
	// Encoder.layout or a key action was placed on an encoder slot),
	// fall back to $X1 ("Icon" layout: title + centered icon) so the
	// LCD renders unstretched content instead of a blank screen.
	$: if (slot && context?.controller === "Encoder" && (slot.feedback_layout ?? "$X1") !== resolvedLayoutId) {
		const pluginId = slot.action.plugin;
		const layoutId = slot.feedback_layout ?? "$X1";
		resolvedLayoutId = layoutId;
		resolveLayout(pluginId, layoutId).then((layout) => {
			if (resolvedLayoutId === layoutId) resolvedLayout = layout;
		});
	}

	listen("key_moved", ({ payload }: { payload: { context: Context; pressed: boolean } }) => {
		if (JSON.stringify(context) == JSON.stringify(payload.context)) pressed = payload.pressed;
	});

	function select(event: MouseEvent | KeyboardEvent) {
		if (event instanceof MouseEvent && event.ctrlKey) return;
		$openContextMenu = null;
		if (!slot) {
			$inspectedInstance = context;
			return;
		}
		if (slot.action.uuid == "opendeck.multiaction" || slot.action.uuid == "opendeck.toggleaction") {
			$inspectedParentAction = context;
		} else {
			$inspectedInstance = slot.context;
		}
	}

	function onfocus() {
		$openContextMenu = null;
		if (!slot) {
			$inspectedInstance = context;
			return;
		}
		if (slot.action.uuid != "opendeck.multiaction" && slot.action.uuid != "opendeck.toggleaction") {
			$inspectedInstance = slot.context;
		} else {
			$inspectedInstance = context;
		}
	}

	let contextMenuEl: HTMLDivElement;
	async function contextMenu(event: MouseEvent | KeyboardEvent) {
		event.preventDefault();
		if (!active || !context) return;
		const rect = canvas.getBoundingClientRect();
		let x = (event instanceof MouseEvent && event.x) ? event.x : rect.left;
		let y = (event instanceof MouseEvent && event.y) ? event.y : rect.bottom;
		$openContextMenu = { context, x, y };
		await tick();
		contextMenuEl?.querySelector("button")?.focus();
	}

	let showEditor = false;
	function edit() {
		$openContextMenu = null;
		showEditor = true;
	}

	function copy() {
		$openContextMenu = null;
		if (!context || !slot) return;
		copiedItem.set({ type: "instance", source: context });
	}

	export let handlePaste: ((item: CopiedItem, destination: Context) => Promise<void>) | undefined = undefined;
	async function paste() {
		$openContextMenu = null;
		if (!$copiedItem || !context || !handlePaste) return;
		await handlePaste($copiedItem, context);
		await tick();
		$inspectedInstance = `${context.device}.${context.profile}.${context.controller}.${context.position}.0`;
	}

	async function clear() {
		$openContextMenu = null;
		if (!slot) return;
		await invoke("remove_instance", { context: slot.context });
		showEditor = false;
		slot = null;
		inslot = slot;
		await tick();
		$inspectedInstance = context;
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
	let previewComposeCanvas: HTMLCanvasElement | undefined;
	let lock = new CanvasLock();
	export let size = 144;
	$: (async () => {
		const sl = structuredClone(slot);
		if (!sl) {
			const unlock = await lock.lock();
			try {
				const ctx = canvas?.getContext("2d");
				if (ctx) ctx.clearRect(0, 0, canvas.width, canvas.height);
				// Encoder slots: profile switch already calls clear_screen
				// on the backend. Individual null pushes here are redundant
				// and race with the plugin's fast-path image pushes.
				if (active && context?.controller !== "Encoder") {
					await invoke("update_image", { context, image: null });
				}
			} finally {
				unlock();
			}
		} else if (context?.controller === "Encoder" && resolvedLayout) {
			// Encoder slot with a feedback layout: composite the layout
			// items and push the rendered 200x100 pixmap to the device.
			// The $X1 "icon" slot falls back to the action's state image
			// when the plugin hasn't pushed its own, matching Elgato's
			// default rendering of the user-assigned icon.
			const unlock = await lock.lock();
			try {
				const feedback = { ...(sl.feedback ?? {}) } as Record<string, unknown>;
				if (feedback.icon == null) {
					const fallback = sl.action.states[sl.current_state]?.image ?? sl.action.icon;
					if (fallback) feedback.icon = fallback;
				}
				if (feedback.title == null && state?.text) feedback.title = state.text;
				// Detect full-canvas data URI pushes. The backend fast path
				// has already sent these directly to the device, so we only
				// need to render the preview (offscreen compose to avoid
				// visible flicker during the async pixmap decode).
				const isFullCanvasFastPath =
					typeof feedback["full-canvas"] === "string" &&
					(feedback["full-canvas"] as string).startsWith("data:");
				if (isFullCanvasFastPath) {
					if (!previewComposeCanvas) previewComposeCanvas = document.createElement("canvas");
					await renderFeedback(previewComposeCanvas, resolvedLayout, feedback);
					const ctx = canvas?.getContext("2d");
					if (ctx) {
						canvas.width = previewComposeCanvas.width;
						canvas.height = previewComposeCanvas.height;
						ctx.drawImage(previewComposeCanvas, 0, 0);
					}
				} else {
					await renderFeedback(canvas, resolvedLayout, feedback);
					if (active) {
						await invoke("update_image", { context, image: canvas.toDataURL("image/png") });
					}
				}
			} finally {
				unlock();
			}
		} else if (context?.controller !== "Encoder") {
			// Non-encoder slots: render via renderImage (key-style square).
			// Encoder slots skip this -- they wait for layout resolution
			// before rendering. The backend fast path handles device updates
			// in the meantime so the encoder LCD isn't blank.
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

	async function triggerVirtualPress() {
		if (!active || !context || !slot) return;
		await invoke("trigger_virtual_press", { context });
	}

	$: accessibleLabel = label + (slot ? ": " + slot.action.name + (state?.show && state?.text ? " - " + state.text : "") : "");
</script>

{#if encoderStrip}
	<div class="flex-1 relative" style="aspect-ratio: 2 / 1; z-index: {slot && $inspectedInstance == slot.context ? 10 : 0};">
		<canvas
			bind:this={canvas}
			class="absolute inset-0 w-full h-full outline-none outline-offset-2 outline-blue-500 transition-colors duration-150"
			class:border-neutral-700={!dragHighlight || dragHighlight === "incompatible"}
			class:border-green-500={dragHighlight === "empty"}
			class:border-orange-500={dragHighlight === "occupied"}
			class:border-blue-400={dragHighlight === "hovered"}
			class:border-y-3={true}
			class:border-l-3={encoderPosition === 0}
			class:border-r-3={encoderPosition === encoderCount - 1}
			class:border-l-[1.5px]={encoderPosition > 0}
			class:border-r-[1.5px]={encoderPosition < encoderCount - 1}
			class:rounded-l-xl={encoderPosition === 0}
			class:rounded-r-xl={encoderPosition === encoderCount - 1}
			class:outline-solid={active && ((slot && $inspectedInstance == slot.context) || (context && $inspectedInstance == context))}
			class:bg-black={slot != null}
			width={200}
			height={100}
			draggable={slot != null}
			{tabindex}
			{role}
			aria-label={accessibleLabel}
			on:dragstart
			on:dragover
			on:dragleave
			on:drop
			on:click|stopPropagation={select}
			on:dblclick|stopPropagation={triggerVirtualPress}
			on:keydown={(e) => {
				if (!active || !context) return;
				if (e.key == "Enter") select(e);
				else if (e.key == "F2") edit();
				else if ((e.ctrlKey || e.metaKey) && e.key == "c") copy();
				else if ((e.ctrlKey || e.metaKey) && e.key == "v") paste();
				else if (e.key == "Delete") clear();
				else if (e.key == "ContextMenu" || (e.shiftKey && e.key == "F10")) contextMenu(e);
			}}
			on:keyup|stopPropagation={(e) => {
				if (!active || !context) return;
				if (e.key == " ") select(e);
			}}
			on:focus={onfocus}
			on:contextmenu={contextMenu}
		/>
	</div>
{:else}
	<div
		class="relative transition-all duration-150"
		class:opacity-30={dragHighlight === "incompatible"}
		class:brightness-125={dragHighlight === "hovered"}
		class:scale-110={dragHighlight === "hovered"}
		style={`transform: scale(${(112 /* desired inner size */ / size) * scale});`}
	>
		<canvas
			bind:this={canvas}
			class="relative border-3 rounded-3xl outline-none transition-colors duration-150"
			class:border-neutral-700={!dragHighlight || dragHighlight === "incompatible"}
			class:border-green-500={dragHighlight === "empty"}
			class:border-orange-500={dragHighlight === "occupied"}
			class:border-blue-400={dragHighlight === "hovered"}
			style={`margin: ${-((size + 3 * 2 /* border */ - 132 /* desired outer size */) / 2)}px;`}
			class:outline-solid={active && !dragHighlight && ((slot && $inspectedInstance == slot.context) || (context && $inspectedInstance == context))}
			class:outline-offset-2={true}
			class:outline-blue-500={!dragHighlight}
			class:rounded-full!={context?.controller == "Encoder"}
			class:bg-black={slot != null}
			width={size}
			height={size}
			draggable={slot != null}
			{tabindex}
			{role}
			aria-label={accessibleLabel}
			on:dragstart
			on:dragover
			on:dragleave
			on:drop
			on:click|stopPropagation={select}
			on:dblclick|stopPropagation={triggerVirtualPress}
			on:keydown={(e) => {
				if (!active || !context) return;
				if (e.key == "Enter") select(e);
				else if (e.key == "F2") edit();
				else if ((e.ctrlKey || e.metaKey) && e.key == "c") copy();
				else if ((e.ctrlKey || e.metaKey) && e.key == "v") paste();
				else if (e.key == "Delete") clear();
				else if (e.key == "ContextMenu" || (e.shiftKey && e.key == "F10")) contextMenu(e);
			}}
			on:keyup|stopPropagation={(e) => {
				if (!active || !context) return;
				if (e.key == " ") select(e);
			}}
			on:focus={onfocus}
			on:contextmenu={contextMenu}
		/>
		{#if isTouchPoint && !slot}
			<div class="absolute left-1/4 top-1/2 w-1/2 border-t-4 border-neutral-700 pointer-events-none"></div>
		{/if}
	</div>
{/if}

{#if $openContextMenu && $openContextMenu?.context == context}
	<div
		bind:this={contextMenuEl}
		class="absolute w-32 font-semibold text-sm text-neutral-300 bg-neutral-700 border border-neutral-600 rounded-lg divide-y divide-neutral-600! z-10"
		style={`left: ${$openContextMenu.x}px; top: ${$openContextMenu.y}px;`}
	>
		{#if !slot}
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-lg cursor-pointer"
				on:click|stopPropagation={paste}
			>
				<Clipboard size="18" class="text-neutral-300" />
				<span class="ml-2"> Paste </span>
			</button>
		{:else}
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-t-lg cursor-pointer"
				on:click|stopPropagation={edit}
			>
				<Pencil size="18" class="text-neutral-300" />
				<span class="ml-2"> Edit </span>
			</button>
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors cursor-pointer"
				on:click|stopPropagation={copy}
			>
				<Copy size="18" class="text-neutral-300" />
				<span class="ml-2"> Copy </span>
			</button>
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-b-lg cursor-pointer"
				on:click|stopPropagation={clear}
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
