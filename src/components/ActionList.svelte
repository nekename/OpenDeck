<script lang="ts">
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
		<details open class="mb-2">
			<summary class="text-xl font-semibold dark:text-neutral-300">{name}</summary>
			{#each actions as action}
				<div class="flex flex-row items-center my-2 space-x-2">
					<img
						src={!action.icon.startsWith("opendeck/") ? "http://localhost:57118/" + action.icon : action.icon.replace("opendeck", "")}
						alt={$localisations?.[action.plugin]?.[action.uuid]?.Tooltip ?? action.tooltip}
						class="w-12 h-12 rounded-xs"
						draggable="true"
						on:dragstart={(event) => event.dataTransfer?.setData("action", JSON.stringify(action))}
					/>
					<span class="dark:text-neutral-400">{$localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name}</span>
				</div>
			{/each}
		</details>
	{/each}
</div>
