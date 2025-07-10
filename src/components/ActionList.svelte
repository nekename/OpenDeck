<script lang="ts">
	import CaretRight from "phosphor-svelte/lib/CaretRight";

	import type { Action } from "$lib/Action";

	import { localisations } from "$lib/settings";

	import { invoke } from "@tauri-apps/api/core";

	let categories: { [name: string]: Action[] } = {};
	export async function reload() {
		categories = await invoke("get_categories");
	}
	reload();
</script>

<div class="grow mt-1 overflow-auto">
	{#each Object.entries(categories).sort((a, b) => a[0] == "OpenDeck" ? -1 : b[0] == "OpenDeck" ? 1 : a[0].localeCompare(b[0])) as [name, actions]}
		<details open class="mb-2 group">
			<summary class="text-xl font-semibold list-none pl-2 p-3 flex items-center gap-4 dark:text-neutral-300 hover:cursor-pointer hover:bg-neutral-200 dark:hover:bg-neutral-800">
				<CaretRight size={22} class="group-open:rotate-90 transition-transform" />
				{name}
			</summary>
			{#each actions as action}
				<div class="flex flex-row items-center pl-6 my-2 space-x-2">
					<img
						src={!action.icon.startsWith("opendeck/") ? "http://localhost:57118/" + action.icon : action.icon.replace("opendeck", "")}
						alt={$localisations?.[action.plugin]?.[action.uuid]?.Tooltip ?? action.tooltip}
						class="w-8 h-8 rounded-xs cursor-grab hover:active:cursor-grabbing"
						draggable="true"
						on:dragstart={(event) => event.dataTransfer?.setData("action", JSON.stringify(action))}
					/>
					<span class="dark:text-neutral-400">{$localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name}</span>
				</div>
			{/each}
		</details>
	{/each}
</div>
