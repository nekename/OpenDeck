<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { ActionState } from "$lib/ActionState";
	import { type Context, actionContextString } from "$lib/Context";
	import type { CopiedItem } from "$lib/propertyInspector";

	import Clipboard from "phosphor-svelte/lib/Clipboard";
	import Copy from "phosphor-svelte/lib/Copy";
	import Pencil from "phosphor-svelte/lib/Pencil";
	import Trash from "phosphor-svelte/lib/Trash";
	import InstanceEditor from "./InstanceEditor.svelte";

	import { t } from "$lib/i18n";
	import { copiedItem, inspectedInstance, inspectedParentAction, openContextMenu } from "$lib/propertyInspector";
	import { CanvasLock, renderImage } from "$lib/rendererHelper";
	import { settings } from "$lib/settings";

	import { invoke } from "@tauri-apps/api/core";
	import { listen, type UnlistenFn } from "@tauri-apps/api/event";
	import { createEventDispatcher, onDestroy, tick } from "svelte";

	const dispatch = createEventDispatcher<{ cleared: void }>();

	export let context: Context | null;
	export let label: string = "";
	export let tabindex: number = 0;
	export let role: string = "gridcell";

	// One-way slot data. Do not bind this from the parent — Svelte writes the
	// child's value into a replaced profile and leaks keys across pages.
	export let inslot: ActionInstance | null;
	let slot: ActionInstance | null;
	// Copy the parent slot only when the button identity changes. A new context object
	// on each parent render must not wipe the image that update_state just delivered.
	let seenSlotId = "";
	$: {
		const slotId = `${context?.page ?? "none"}:${context?.controller ?? ""}:${context?.position ?? ""}:${inslot?.context ?? "empty"}`;
		if (slotId !== seenSlotId && !(inslot && context && inslot.context.split(".")[0] != context.device)) {
			seenSlotId = slotId;
			slot = inslot ?? null;
		}
	}

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

	function payloadContextString(payloadContext: string | Context): string {
		return typeof payloadContext == "string" ? payloadContext : actionContextString(payloadContext);
	}

	const unlistens: Promise<UnlistenFn>[] = [];
	unlistens.push(
		listen("update_state", ({ payload }: { payload: { context: string; contents: ActionInstance | null } }) => {
			if (!context) return;
			if (payloadContextString(payload.context) != actionContextString(context, 0)) return;
			// Parent slot is empty. A late setImage from the previous page must not fill this key.
			if (!inslot && payload.contents) return;
			slot = payload.contents;
			// Dial feedback lives in the backend renderer, outside drawKey's state fields.
			if (context.controller === "Encoder") scheduleDraw();
		}),
	);

	unlistens.push(
		listen("key_moved", ({ payload }: { payload: { context: Context; pressed: boolean } }) => {
			if (JSON.stringify(context) == JSON.stringify(payload.context)) pressed = payload.pressed;
		}),
	);

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
		let x = event instanceof MouseEvent && event.x ? event.x : rect.left;
		let y = event instanceof MouseEvent && event.y ? event.y : rect.bottom;
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
		$inspectedInstance = actionContextString(context);
	}

	async function clear() {
		$openContextMenu = null;
		if (!slot) return;
		const parsed = slot.context.split(".");
		if (context && parsed.length >= 6 && parsed[2] != String(context.page)) {
			slot = null;
			dispatch("cleared");
			return;
		}
		await invoke("remove_instance", { context: slot.context });
		showEditor = false;
		slot = null;
		dispatch("cleared");
		await tick();
		$inspectedInstance = context;
	}

	let showAlert: boolean = false;
	let showOk: boolean = false;
	let timeouts: number[] = [];
	unlistens.push(
		listen("show_alert", ({ payload }: { payload: string }) => {
			if (!slot || payload != slot.context) return;
			timeouts.forEach(clearTimeout);
			showOk = false;
			showAlert = true;
			timeouts.push(setTimeout(() => (showAlert = false), 1.5e3));
		}),
	);
	unlistens.push(
		listen("show_ok", ({ payload }: { payload: string }) => {
			if (!slot || payload != slot.context) return;
			timeouts.forEach(clearTimeout);
			showAlert = false;
			showOk = true;
			timeouts.push(setTimeout(() => (showOk = false), 1.5e3));
		}),
	);

	onDestroy(() => {
		unlistens.forEach((unlisten) => unlisten.then((fn) => fn()));
	});

	let canvas: HTMLCanvasElement;
	let lock = new CanvasLock();
	export let size = 144;
	// Canvas resolution defaults to a square `size`, but rectangular controllers (e.g. the Neo's infobar) can override this.
	export let width: number = size;
	export let height: number = size;
	let drawGen = 0;
	let drawQueued = false;
	function scheduleDraw() {
		if (drawQueued) return;
		drawQueued = true;
		requestAnimationFrame(() => {
			drawQueued = false;
			void draw();
		});
	}
	async function draw() {
		const gen = ++drawGen;
		const live = () => gen === drawGen;
		const drawContext = context;
		if (!canvas) await tick();
		if (!live() || !canvas) return;
		const sl = slot ? structuredClone(slot) : null;
		// Only an empty slot clears. A missing state must not wipe a button that is still there.
		if (!sl) {
			canvas.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height);
			if (active && drawContext && live()) await invoke("update_image", { context: drawContext, image: null });
			return;
		}
		const currentState = sl.states?.[sl.current_state];
		if (!currentState || !live()) return;
		const unlock = await lock.lock();
		try {
			if (!live() || !canvas) return;
			let fallback = sl.action.states[sl.current_state]?.image ?? sl.action.icon;
			await renderImage(
				canvas,
				drawContext,
				currentState,
				fallback,
				showOk,
				showAlert,
				true,
				active,
				pressed,
				$settings?.rotation,
				live,
				sl.context,
				sl.current_state ?? 0,
			);
		} finally {
			unlock();
		}
	}
	$: drawKey = [
		slot?.context ?? "",
		slot?.current_state ?? "",
		state?.image ?? "",
		state?.image_override ?? "",
		state?.text ?? "",
		state?.text_override ?? "",
		state?.background_colour ?? "",
		state?.show ?? "",
		showOk,
		showAlert,
		pressed,
		active,
		$settings?.rotation ?? "",
		context?.page ?? "",
		context?.controller ?? "",
		context?.position ?? "",
	].join("\u001f");
	$: {
		void drawKey;
		scheduleDraw();
	}

	function clearAndRedraw() {
		canvas?.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height);
		scheduleDraw();
	}
	$: if ($settings?.rotation != undefined) {
		clearAndRedraw();
	}

	async function triggerVirtualPress() {
		if (!active || !context || !slot) return;
		await invoke("trigger_virtual_press", { context });
	}

	$: accessibleLabel =
		label + (slot ? ": " + slot.action.name + (state?.show && (state?.text_override || state?.text) ? " - " + (state.text_override || state.text) : "") : "");
</script>

<div class="relative" style={`transform: scale(${(112 /* desired inner size */ / size) * scale});`}>
	<canvas
		bind:this={canvas}
		class="relative border-3 border-neutral-700 rounded-3xl outline-none outline-offset-2 outline-blue-500"
		style={`margin: ${-((size + 3 * 2 /* border */ - 132) /* desired outer size */ / 2)}px;`}
		class:outline-solid={active && ((slot && $inspectedInstance == slot.context) || (context && $inspectedInstance == context))}
		class:rounded-full!={context?.controller == "Encoder"}
		class:rounded-lg!={context?.controller == "Infobar"}
		class:bg-black={slot != null}
		{width}
		{height}
		draggable={slot != null}
		{tabindex}
		{role}
		aria-label={accessibleLabel}
		on:dragstart
		on:dragover
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

{#if $openContextMenu && $openContextMenu?.context == context}
	<div
		bind:this={contextMenuEl}
		class="absolute w-32 font-semibold text-sm text-neutral-300 bg-neutral-700 border border-neutral-600 rounded-lg divide-y divide-neutral-600! z-10"
		style={`left: ${$openContextMenu.x}px; top: ${$openContextMenu.y}px;`}
	>
		{#if !slot}
			<button class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-lg cursor-pointer" on:click|stopPropagation={paste}>
				<Clipboard size="18" class="text-neutral-300" />
				<span class="ml-2">{$t("key.paste")}</span>
			</button>
		{:else}
			<button class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-t-lg cursor-pointer" on:click|stopPropagation={edit}>
				<Pencil size="18" class="text-neutral-300" />
				<span class="ml-2">{$t("key.edit")}</span>
			</button>
			<button class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors cursor-pointer" on:click|stopPropagation={copy}>
				<Copy size="18" class="text-neutral-300" />
				<span class="ml-2">{$t("key.copy")}</span>
			</button>
			<button class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-b-lg cursor-pointer" on:click|stopPropagation={clear}>
				<Trash size="18" class="text-red-400" />
				<span class="ml-2">{$t("key.delete")}</span>
			</button>
		{/if}
	</div>
{/if}

{#if slot && showEditor}
	<InstanceEditor bind:instance={slot} bind:showEditor />
{/if}
