<script lang="ts">
	import { t } from "$lib/i18n";
	import { PRODUCT_NAME } from "$lib/singletons";

	import { invoke } from "@tauri-apps/api/core";

	let buildInfo: string;
	(async () => buildInfo = await invoke("get_build_info"))();
</script>

<div class="flex flex-col justify-center items-center w-full h-full text-center text-neutral-300">
	<div class="w-80 text-sm">
		<h2 class="text-lg font-bold mb-2">{$t("no_devices_detected.title")}</h2>
		<p class="mb-2">{$t("no_devices_detected.check_connection")}</p>
		{#if buildInfo?.split("</summary>")[0]?.includes("linux")}
			<p class="mb-2">{$t("no_devices_detected.check_udev")}</p>
		{/if}
		<p class="mb-4">{$t("no_devices_detected.install_plugin")}</p>
		<button
			class="px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
			on:click={() => invoke("restart")}
		>
			{$t("no_devices_detected.restart", { PRODUCT_NAME })}
		</button>
	</div>
</div>
