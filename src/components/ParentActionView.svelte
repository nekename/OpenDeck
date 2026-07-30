<script lang="ts">
	import type { Action } from "$lib/Action";
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Profile } from "$lib/Profile";

	import Trash from "phosphor-svelte/lib/Trash";
	import Key from "./Key.svelte";

	import { t } from "$lib/i18n";
	import { copiedItem, inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";
	import { onMount, tick } from "svelte";

	export let profile: Profile;

	let listEl: HTMLDivElement;
	onMount(() => {
		const first = listEl?.querySelector("[role='listitem']") as HTMLElement | null;
		first?.focus();
	});

	let children: ActionInstance[];
	$: children = profile.keys[$inspectedParentAction!.position]!.children!;
	let parentUuid: string;
	$: parentUuid = profile.keys[$inspectedParentAction!.position]!.action.uuid;
	let parentContext: string;
	$: parentContext = profile.keys[$inspectedParentAction!.position]!.context;
	let parentSettings: any;
	$: parentSettings = profile.keys[$inspectedParentAction!.position]!.settings;
	let parentName: string;
	$: parentName =
		parentUuid == "opendeck.toggleaction"
			? $t("parent_action_view.toggle")
			: parentUuid == "opendeck.doubleclickaction"
				? $t("parent_action_view.double_click")
				: $t("parent_action_view.multi");

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		if (event.dataTransfer?.types.includes("action")) event.dataTransfer.dropEffect = "copy";
	}

	async function addAction(action: Action) {
		if (
			(parentUuid == "opendeck.multiaction" && !action.supported_in_multi_actions) ||
			// Built-in actions are handled natively rather than over WebSocket, so they cannot be children of other actions.
			((parentUuid == "opendeck.toggleaction" || parentUuid == "opendeck.doubleclickaction") && action.plugin == "opendeck") ||
			(parentUuid == "opendeck.doubleclickaction" && children.length >= 2)
		) {
			return;
		}
		let response: ActionInstance | null = await invoke("create_instance", { context: $inspectedParentAction, action });
		if (response) profile.keys[$inspectedParentAction!.position] = response;
	}

	async function handleDrop({ dataTransfer }: DragEvent) {
		if (dataTransfer?.getData("action")) {
			let action = JSON.parse(dataTransfer?.getData("action"));
			await addAction(action);
		}
	}

	async function handlePaste() {
		if (!$copiedItem || $copiedItem.type != "action") return;
		await addAction($copiedItem.action);
	}

	async function removeInstance(index: number, refocus = false) {
		await invoke("remove_instance", { context: children[index].context });
		children.splice(index, 1);
		profile.keys[$inspectedParentAction!.position]!.children = children;

		if (index == 0) {
			profile.keys[$inspectedParentAction!.position]!.settings.delays?.splice(0, 1);
		} else {
			profile.keys[$inspectedParentAction!.position]!.settings.delays?.splice(index - 1, 1);
		}

		if (!refocus) return;

		await tick();
		const items = Array.from(listEl?.querySelectorAll("[role='listitem']") ?? []) as HTMLElement[];
		if (items.length == 0) return;

		const targetIndex = children.length == 0 ? 0 : Math.min(index, children.length - 1);
		for (let i = 0; i < items.length; i++) {
			items[i].tabIndex = i == targetIndex ? 0 : -1;
		}
		items[targetIndex]?.focus();
	}

	async function setDelay(index: number, event: Event) {
		const target = event.currentTarget as HTMLInputElement;
		const val = Math.max(0, parseInt(target.value) || 0);
		const settings = await invoke<any>("set_child_delay", { parentContext, index, delayMs: val });
		profile.keys[$inspectedParentAction!.position]!.settings = settings;
	}

	async function setDoubleClickWindow(event: Event) {
		const target = event.currentTarget as HTMLInputElement;
		const val = Math.max(100, parseInt(target.value) || 400);
		const settings = await invoke<any>("set_double_click_window", { parentContext, windowMs: val });
		profile.keys[$inspectedParentAction!.position]!.settings = settings;
	}

	function handleListKeydown(event: KeyboardEvent) {
		if (!["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
		const list = event.currentTarget as HTMLElement;
		const items = Array.from(list.querySelectorAll("[role='listitem']"));
		const currentIndex = items.indexOf(document.activeElement?.closest("[role='listitem']") as Element);
		if (currentIndex == -1) return;

		event.preventDefault();

		let newIndex = currentIndex;
		switch (event.key) {
			case "ArrowDown":
				newIndex = Math.min(currentIndex + 1, items.length - 1);
				break;
			case "ArrowUp":
				newIndex = Math.max(currentIndex - 1, 0);
				break;
			case "Home":
				newIndex = 0;
				break;
			case "End":
				newIndex = items.length - 1;
				break;
		}

		if (newIndex == currentIndex) return;
		(items[currentIndex] as HTMLElement).tabIndex = -1;
		(items[newIndex] as HTMLElement).tabIndex = 0;
		(items[newIndex] as HTMLElement).focus();
	}
</script>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") $inspectedParentAction = null;
	}}
/>

<div class="px-6 pt-6 pb-4 text-neutral-300">
	<button class="float-right text-xl" on:click={() => ($inspectedParentAction = null)} aria-label={$t("settings.close")}>✕</button>
	<h1 class="font-semibold text-2xl">{parentName}</h1>
</div>

{#if parentUuid == "opendeck.doubleclickaction"}
	<div class="flex flex-row items-center gap-2 mx-4 mb-1 px-3 py-2 bg-neutral-800 border border-dashed border-neutral-600 rounded-lg">
		<span class="text-xs text-neutral-400">{$t("parent_action_view.window.label")}</span>
		<input
			type="number"
			min="100"
			max="2000"
			step="50"
			value={parentSettings?.double_click_window ?? 400}
			on:input={setDoubleClickWindow}
			class="no-spinner w-20 px-1 py-0.5 text-center text-sm text-neutral-300 bg-neutral-900 border border-neutral-600 rounded"
			aria-label={$t("parent_action_view.window.label")}
		/>
		<span class="text-xs text-neutral-500">ms</span>
	</div>
{/if}

<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
<div
	bind:this={listEl}
	class="flex flex-col h-128 overflow-auto"
	on:click={() => ($inspectedInstance = null)}
	role="list"
	aria-label="{parentName} {$t('parent_action_view.children')}"
	on:keydown={handleListKeydown}
>
	{#each children as instance, index}
		<!-- svelte-ignore a11y-no-noninteractive-tabindex a11y-no-noninteractive-element-interactions -->
		<div
			class="flex flex-row items-center mx-4 my-1 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg focus-within:outline-solid focus-within:outline-offset-2 focus-within:outline-blue-500"
			class:my-2={parentUuid == "opendeck.toggleaction"}
			on:click|stopPropagation={() => ($inspectedInstance = instance.context)}
			on:focus|stopPropagation={() => ($inspectedInstance = instance.context)}
			on:keydown={(e) => {
				if (e.key == "Delete") removeInstance(index, true);
			}}
			role="listitem"
			tabindex={index == 0 ? 0 : -1}
		>
			<Key
				inslot={instance}
				context={null}
				active={false}
				scale={3 / 4}
				role="presentation"
				tabindex={-1}
				label={parentName + " " + $t("parent_action_view.child") + " " + (index + 1)}
			/>
			<p class="ml-4 text-xl text-neutral-300">
				{#if parentUuid == "opendeck.doubleclickaction"}
					<span class="block text-sm text-neutral-400"
						>{index == 0 ? $t("parent_action_view.single_click_label") : $t("parent_action_view.double_click_label")}</span
					>
				{/if}
				{instance.action.name}
			</p>
			<button
				class="ml-auto mr-10"
				on:click|stopPropagation={() => removeInstance(index)}
				tabindex={-1}
				aria-label={$t("parent_action_view.remove", { name: instance.action.name })}
			>
				<Trash size="32" class="text-neutral-400" />
			</button>
		</div>

		{#if parentUuid == "opendeck.multiaction" && index < children.length - 1}
			<div class="flex flex-row items-center gap-2 mx-14 my-1 px-3 py-2 bg-neutral-800 border border-dashed border-neutral-600 rounded-lg">
				<span class="text-xs text-neutral-400">{$t("parent_action_view.delay.label")}</span>
				<input
					type="number"
					min="0"
					max="300000"
					step="100"
					value={parentSettings?.delays?.[index] ?? 100}
					on:input={(e) => setDelay(index, e)}
					class="no-spinner w-20 px-1 py-0.5 text-center text-sm text-neutral-300 bg-neutral-900 border border-neutral-600 rounded"
					aria-label={$t("parent_action_view.delay.aria", { name: children[index + 1].action.name })}
				/>
				<span class="text-xs text-neutral-500">ms</span>
			</div>
		{/if}
	{/each}
	{#if !(parentUuid == "opendeck.doubleclickaction" && children.length >= 2)}
		<!-- svelte-ignore a11y-no-noninteractive-tabindex a11y-no-noninteractive-element-interactions -->
		<div
			class="flex flex-row items-center mx-4 mt-2 mb-4 p-3 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-dashed border-neutral-600 rounded-lg focus-within:outline-solid focus-within:outline-offset-2 focus-within:outline-blue-500"
			on:dragover={handleDragOver}
			on:drop={handleDrop}
			on:click={() => ($inspectedInstance = null)}
			on:focus={() => ($inspectedInstance = null)}
			on:keydown={(e) => {
				if ((e.ctrlKey || e.metaKey) && e.key == "v") handlePaste();
			}}
			role="listitem"
			tabindex={children.length == 0 ? 0 : -1}
			aria-label={$t("parent_action_view.drag_copy")}
		>
			<img src="/cube.png" class="m-2 w-24 rounded-xl" alt="" />
			<p class="ml-4 text-xl text-neutral-400">{$t("parent_action_view.drag_paste")}</p>
		</div>
	{/if}
</div>

<style>
	:global(.no-spinner::-webkit-outer-spin-button),
	:global(.no-spinner::-webkit-inner-spin-button) {
		-webkit-appearance: none;
		margin: 0;
	}
	:global(.no-spinner[type="number"]) {
		-moz-appearance: textfield;
		appearance: textfield;
	}
</style>
