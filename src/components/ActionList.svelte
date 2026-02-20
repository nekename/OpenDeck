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

<div class="flex flex-col w-[18rem] h-full p-2 border-l border-neutral-700">
	<div class="flex flex-row items-center bg-neutral-700 border-2 border-neutral-900 rounded-md">
		<MagnifyingGlass size="13" class="ml-2 mr-1 text-neutral-300" />
		<input
			bind:value={query}
			class="w-full p-1 text-sm text-neutral-300 outline-hidden"
			placeholder="Search actions"
			type="search"
			spellcheck="false"
		/>
	</div>

	<div class="grow mt-1 overflow-auto select-none">
		{#each filteredCategories as [name, { icon, actions }]}
			<details open class="mb-2">
				<summary class="text-xl font-semibold text-neutral-300">
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
						class="flex flex-row items-center my-2 space-x-2"
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
							class="w-12 h-12 rounded-xs pointer-events-none"
						/>
						<span class="text-neutral-400">{$localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name}</span>
					</div>
				{/each}
			</details>
		{/each}
	</div>
</div>
