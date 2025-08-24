<script lang="ts">
	import type { Action } from "$lib/Action";

	import { localisations } from "$lib/settings";

	import { invoke } from "@tauri-apps/api/core";
	import { createEventDispatcher } from "svelte";

	const dispatch = createEventDispatcher();

	let categories: { [name: string]: { icon?: string; actions: Action[] } } = {};
	let plugins: any[] = [];
	export async function reload() {
		categories = await invoke("get_categories");
		plugins = await invoke("list_plugins");
	}
	reload();

	// Accessibility state for keyboard-based action assignment
	let selectedAction: Action | null = null;
	let focusedActionIndex: number = -1;
	let allActions: Action[] = [];
	let announcementEl: HTMLElement;

	// Update flattened action list when categories change
	$: {
		allActions = Object.entries(categories)
			.sort((a, b) => a[0] == "OpenDeck" ? -1 : b[0] == "OpenDeck" ? 1 : a[0].localeCompare(b[0]))
			.flatMap(([_, { actions }]) => actions);
	}

	function announceToScreenReader(message: string) {
		if (announcementEl) {
			announcementEl.textContent = message;
		}
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (allActions.length === 0) return;

		switch (event.key) {
			case "ArrowDown":
				event.preventDefault();
				focusedActionIndex = Math.min(focusedActionIndex + 1, allActions.length - 1);
				announceToScreenReader(
					`${$localisations?.[allActions[focusedActionIndex].plugin]?.[allActions[focusedActionIndex].uuid]?.Name ?? allActions[focusedActionIndex].name}. ${
						focusedActionIndex + 1
					} of ${allActions.length}`,
				);
				break;
			case "ArrowUp":
				event.preventDefault();
				focusedActionIndex = Math.max(focusedActionIndex - 1, 0);
				announceToScreenReader(
					`${$localisations?.[allActions[focusedActionIndex].plugin]?.[allActions[focusedActionIndex].uuid]?.Name ?? allActions[focusedActionIndex].name}. ${
						focusedActionIndex + 1
					} of ${allActions.length}`,
				);
				break;
			case "Enter":
			case " ":
				event.preventDefault();
				if (focusedActionIndex >= 0 && focusedActionIndex < allActions.length) {
					selectAction(allActions[focusedActionIndex]);
				}
				break;
			case "Escape":
				event.preventDefault();
				clearSelection();
				break;
		}
	}

	function selectAction(action: Action) {
		selectedAction = action;
		announceToScreenReader(`${$localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name} selected. Use arrow keys to navigate to a key and press Enter to assign this action.`);

		// Dispatch custom event to notify parent components
		dispatch("actionSelected", action);
	}

	function clearSelection() {
		selectedAction = null;
		focusedActionIndex = -1;
		announceToScreenReader("Action selection cleared.");

		// Dispatch custom event to notify parent components
		dispatch("actionCleared");
	}

	// Export function to get selected action for other components
	export function getSelectedAction() {
		return selectedAction;
	}

	export function clearSelectedAction() {
		clearSelection();
	}
</script>

<div
	class="grow mt-1 overflow-auto select-none"
	tabindex="0"
	role="listbox"
	aria-label="Available actions. Use arrow keys to navigate, Enter or Space to select an action, then navigate to a key to assign it."
	on:keydown={handleKeyDown}
>
	<!-- Screen reader announcements -->
	<div bind:this={announcementEl} class="sr-only" aria-live="polite" aria-atomic="true"></div>

	{#if selectedAction}
		<div class="mb-2 p-2 bg-blue-100 dark:bg-blue-900 rounded-md border border-blue-300 dark:border-blue-700" role="status" aria-live="polite">
			<div class="flex flex-row items-center space-x-2">
				<img
					src={!selectedAction.icon.startsWith("opendeck/") ? "http://localhost:57118/" + selectedAction.icon : selectedAction.icon.replace("opendeck", "")}
					alt=""
					class="w-6 h-6 rounded-xs"
				/>
				<span class="text-sm font-medium dark:text-neutral-200">Selected: {$localisations?.[selectedAction.plugin]?.[selectedAction.uuid]?.Name ?? selectedAction.name}</span>
				<button
					class="ml-auto text-xs px-2 py-1 bg-neutral-200 dark:bg-neutral-700 rounded hover:bg-neutral-300 dark:hover:bg-neutral-600"
					on:click={clearSelection}
					aria-label="Clear action selection"
				>
					✕
				</button>
			</div>
		</div>
	{/if}

	{#each Object.entries(categories).sort((a, b) => a[0] == "OpenDeck" ? -1 : b[0] == "OpenDeck" ? 1 : a[0].localeCompare(b[0])) as [name, { icon, actions }], categoryIndex}
		<details open class="mb-2">
			<summary class="text-xl font-semibold dark:text-neutral-300">
				{#if icon || (actions[0] && plugins.find((x) => x.id == actions[0].plugin) && actions.every((x) => x.plugin == actions[0].plugin))}
					<img
						src={icon
							? (!icon.startsWith("opendeck/") ? "http://localhost:57118/" + icon : icon.replace("opendeck", ""))
							: "http://localhost:57118/" + plugins.find((x) => x.id == actions[0].plugin).icon}
						alt={name}
						class="w-5 h-5 rounded-xs ml-1 -mt-1 inline"
					/>
				{/if}
				<span class="ml-1">{name}</span>
			</summary>
			{#each actions as action, actionIndex}
				{@const 			globalIndex = Object.entries(categories)
				.sort((a, b) => a[0] == "OpenDeck" ? -1 : b[0] == "OpenDeck" ? 1 : a[0].localeCompare(b[0]))
				.slice(0, categoryIndex)
				.reduce((sum, [_, { actions }]) => sum + actions.length, 0) + actionIndex}
				<div
					class="flex flex-row items-center my-2 space-x-2 cursor-pointer rounded-md p-1 transition-colors"
					class:bg-blue-100={focusedActionIndex === globalIndex}
					class:dark:bg-blue-900={focusedActionIndex === globalIndex}
					class:bg-blue-200={selectedAction === action}
					class:dark:bg-blue-800={selectedAction === action}
					role="option"
					aria-selected={selectedAction === action}
					tabindex="-1"
					draggable="true"
					title={$localisations?.[action.plugin]?.[action.uuid]?.Tooltip ?? action.tooltip}
					on:dragstart={(event) => event.dataTransfer?.setData("action", JSON.stringify(action))}
					on:click={() => selectAction(action)}
					on:keydown={(event) => {
						if (event.key === "Enter" || event.key === " ") {
							event.preventDefault();
							selectAction(action);
						}
					}}
				>
					<img
						src={!action.icon.startsWith("opendeck/") ? "http://localhost:57118/" + action.icon : action.icon.replace("opendeck", "")}
						alt=""
						class="w-12 h-12 rounded-xs"
					/>
					<span class="dark:text-neutral-400">{$localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name}</span>
				</div>
			{/each}
		</details>
	{/each}
</div>

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
