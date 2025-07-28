<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
</script>

<div class="flex flex-col justify-center items-center w-full h-full text-center dark:text-neutral-300">
	<div class="w-80 text-sm">
		<h2 class="text-lg font-bold mb-2">No devices detected</h2>
		<p class="mb-2">Make sure your devices are connected properly and you have permission to access them.</p>
		<p class="mb-4">
			On GNU+Linux, ensure you have the <button on:click={() => invoke("open_url", { url: "https://github.com/nekename/OpenDeck?tab=readme-ov-file#linux" })} class="underline">
				correct udev subsystem rules
			</button> installed.
		</p>
		<button
			class="px-2 py-1 text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 hover:bg-neutral-200 dark:bg-neutral-700 dark:hover:bg-neutral-800 border dark:border-neutral-600 rounded-lg"
			on:click={() => invoke("restart")}
		>
			Restart OpenDeck
		</button>
	</div>
</div>
