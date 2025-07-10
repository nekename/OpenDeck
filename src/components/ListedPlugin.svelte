<script lang="ts">
	import WarningCircle from "phosphor-svelte/lib/WarningCircle";

	import { invoke } from "@tauri-apps/api/core";

	export let icon: string;
	export let name: string;
	export let subtitle: string;
	export let hidden: boolean = false;
	export let disconnected: boolean = false;
	export let action: () => void;
</script>

<div
	class="flex flex-row items-center m-2 p-2 bg-neutral-200 dark:bg-neutral-700 rounded-md"
	class:hidden
>
	<img src={icon} class="w-24 h-24 rounded-md" class:opacity-75={disconnected} alt={name} loading="lazy" />
	<div class="ml-4 mr-2 dark:text-neutral-300 [overflow-wrap:anywhere]" class:opacity-75={disconnected}>
		<p class="font-semibold">{name}</p> {subtitle}
	</div>

	<div class="flex flex-col ml-auto mr-4 space-y-2">
		{#if disconnected}
			<button on:click={() => invoke("open_log_directory")} class="group">
				<WarningCircle size="24" class="fill-yellow-500 hover:fill-yellow-600" />
			</button>
		{/if}
		<button on:click={action} class="hover:bg-neutral-300 dark:hover:bg-neutral-800 p-2 rounded-full">
			<slot />
		</button>
	</div>
</div>
