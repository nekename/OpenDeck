<script lang="ts">
	import { PRODUCT_NAME } from "$lib/singletons";

	import { invoke } from "@tauri-apps/api/core";

	let buildInfo: string;
	(async () => buildInfo = await invoke("get_build_info"))();
</script>

<div class="flex flex-col justify-center items-center w-full h-full text-center text-neutral-300">
	<div class="w-80 text-sm">
		<h2 class="text-lg font-bold mb-2">No devices detected</h2>
		<p class="mb-2">Make sure your devices are connected properly and you have permission to access them.</p>
		{#if buildInfo?.includes("linux")}
			<p class="mb-2">Ensure you have the correct udev subsystem rules installed.</p>
		{/if}
		<p class="mb-4">You may need to install a plugin that adds support for your device.</p>
		<button
			class="px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
			on:click={() => invoke("restart")}
		>
			Restart {PRODUCT_NAME}
		</button>
	</div>
</div>
