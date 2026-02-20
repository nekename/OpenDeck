<script lang="ts">
	import type { Action } from "$lib/Action";

	import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";

	import { getWebserverUrl } from "$lib/ports";
	import { localisations } from "$lib/settings";
	import { PRODUCT_NAME } from "$lib/singletons";

	import { invoke } from "@tauri-apps/api/core";

	let categories: { [name: string]: { icon?: string; actions: Action[] } } = {};
	let plugins: any[] = [];
	export async function reload() {
		categories = await invoke("get_categories");
		plugins = await invoke("list_plugins");
	}
	reload();

	let query: string = "";
	let filteredCategories: [string, { icon?: string; actions: Action[] }][] = [];
	$: {
		let lowerCaseQuery = query.toLowerCase().trim();
		filteredCategories = Object.entries(categories)
			.sort((a, b) => a[0] == PRODUCT_NAME ? -1 : b[0] == PRODUCT_NAME ? 1 : a[0].localeCompare(b[0]))
			.map(([categoryName, { icon, actions }]): [string, { icon?: string; actions: Action[] }] => {
				if (!categoryName.toLowerCase().includes(lowerCaseQuery)) {
					actions = actions.filter((action) => action.name.toLowerCase().includes(lowerCaseQuery));
				}
				return [categoryName, { icon, actions }];
			})
			.filter(([_, { actions }]) => actions.length > 0);
	}
</script>

<div class="flex flex-col w-[18rem] h-full bg-neutral-900 border-l border-neutral-700">
	<div class="flex flex-row items-center m-2 bg-neutral-700 border border-neutral-600 rounded-lg">
		<MagnifyingGlass size="13" class="ml-2 mr-1 text-neutral-300" />
		<input
			bind:value={query}
			class="w-full p-1 text-sm text-neutral-300 outline-hidden"
			placeholder="Search actions"
			type="search"
			spellcheck="false"
		/>
	</div>

	<div class="grow overflow-auto select-none divide-y divide-neutral-800!">
		{#each filteredCategories as [name, { icon, actions }]}
			<details open>
				<summary class="pl-4 py-3 text-lg font-semibold text-neutral-300 hover:bg-neutral-800 transition-colors cursor-pointer">
					{#if icon || (actions[0] && plugins.find((x) => x.id == actions[0].plugin) && categories[name].actions.every((x) => x.plugin == actions[0].plugin))}
						<img
							src={icon ? (!icon.startsWith("opendeck/") ? getWebserverUrl(icon) : icon.replace("opendeck", "")) : getWebserverUrl(plugins.find((x) => x.id == actions[0].plugin).icon)}
							alt={name}
							class="w-5 h-5 rounded-xs ml-1 -mt-1 inline"
						/>
					{/if}
					<span class="ml-1">{name}</span>
				</summary>
				{#each actions as action}
					<div
						class="flex flex-row items-center p-2 pl-6 bg-neutral-950 hover:bg-neutral-900 transition-colors border-t border-neutral-800 cursor-grab active:cursor-grabbing"
						role="group"
						draggable="true"
						title={$localisations?.[action.plugin]?.[action.uuid]?.Tooltip ?? action.tooltip}
						on:dragstart={(event) => {
							if (!event.dataTransfer) return;
							event.dataTransfer.effectAllowed = "copy";
							event.dataTransfer.setData("action", JSON.stringify(action));
						}}
					>
						<img
							src={!action.icon.startsWith("opendeck/") ? getWebserverUrl(action.icon) : action.icon.replace("opendeck", "")}
							alt={$localisations?.[action.plugin]?.[action.uuid]?.Tooltip ?? action.tooltip}
							class="m-0.5 mr-3 w-11 h-11 rounded-lg border border-neutral-700 pointer-events-none"
						/>
						<span class="text-neutral-400">{$localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name}</span>
					</div>
				{/each}
			</details>
		{/each}
	</div>
</div>
