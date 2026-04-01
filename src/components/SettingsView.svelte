<script lang="ts">
	import Heart from "phosphor-svelte/lib/Heart";
	import Star from "phosphor-svelte/lib/Star";
	import Popup from "./Popup.svelte";
	import Tooltip from "./Tooltip.svelte";

	import { settings } from "$lib/settings";
	import { PRODUCT_NAME } from "$lib/singletons";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";

	let showPopup: boolean;
	let buildInfo: string;
	(async () => buildInfo = await invoke("get_build_info"))();

	listen("device_brightness", ({ payload }: { payload: { action: string; value: number } }) => {
		if (!$settings) return;
		let value = $settings.brightness;
		switch (payload.action) {
			case "increase":
				value += payload.value;
				break;
			case "decrease":
				value -= payload.value;
				break;
			default:
				value = payload.value;
				break;
		}
		$settings.brightness = Math.max(0, Math.min(100, value));
	});
</script>

<button
	class="px-3 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
	on:click={() => showPopup = true}
>
	Settings
</button>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") showPopup = false;
	}}
/>

<Popup show={showPopup} label="Settings">
	<button class="mr-2 my-1 float-right text-xl text-neutral-300" on:click={() => showPopup = false} aria-label="Close">✕</button>
	<h2 class="m-2 font-semibold text-xl text-neutral-300">Settings</h2>
	{#if $settings}
		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-language" class="text-neutral-400">Language:</label>
			<div class="select-wrapper">
				<select bind:value={$settings.language} class="w-32" id="settings-language">
					<option value="en">English</option>
					<option value="es">Español</option>
					<option value="zh_CN">中文</option>
					<option value="fr">Français</option>
					<option value="de">Deutsch</option>
					<option value="ja">日本語</option>
					<option value="ko">韓国語</option>
				</select>
			</div>
			<Tooltip>
				{PRODUCT_NAME} itself is not yet translated. Changing this setting will translate the text from installed plugins into your language for those that support it.
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-brightness" class="text-neutral-400">Device brightness:</label>
			<input type="range" min="0" max="100" bind:value={$settings.brightness} id="settings-brightness" />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-sleep_timeout_minutes" class="text-neutral-400">Sleep after inactivity:</label>
			<input type="number" min="0" bind:value={$settings.sleep_timeout_minutes} class="w-12 px-1 text-neutral-300 border border-neutral-600 rounded-lg" id="settings-sleep_timeout_minutes" />
			<span class="text-neutral-400">minutes</span>
			<Tooltip> This option controls how many minutes of inactivity will cause devices to enter sleep mode, where a value of 0 disables sleeping automatically. </Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-rotation" class="text-neutral-400">Image rotation:</label>
			<input type="range" min="0" max="270" step="90" bind:value={$settings.rotation} id="settings-rotation" />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-background" class="text-neutral-400">Run in background:</label>
			<input type="checkbox" bind:checked={$settings.background} id="settings-background" />
			<Tooltip> If this option is enabled, {PRODUCT_NAME} will minimise to the tray and run in the background. </Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-autolaunch" class="text-neutral-400">Start at login:</label>
			<input type="checkbox" bind:checked={$settings.autolaunch} id="settings-autolaunch" />
			<Tooltip>
				If this option is enabled, {PRODUCT_NAME} will automatically start at login.
				{#if buildInfo?.split("</summary>")[0]?.includes("linux")}
					<br />
					If you used Flatpak to install {PRODUCT_NAME}, this option may not function as intended.
				{/if}
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-updatecheck" class="text-neutral-400">Check for updates:</label>
			<input type="checkbox" bind:checked={$settings.updatecheck} id="settings-updatecheck" />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-statistics" class="text-neutral-400">Contribute statistics:</label>
			<input type="checkbox" bind:checked={$settings.statistics} id="settings-statistics" />
		</div>

		{#if !buildInfo?.split("</summary>")[0]?.includes("windows")}
			<div class="flex flex-row items-center m-2 space-x-2">
				<label for="settings-separatewine" class="text-neutral-400">Create separate Wine prefixes:</label>
				<input type="checkbox" bind:checked={$settings.separatewine} id="settings-separatewine" />
				<Tooltip>
					If this option is enabled, {PRODUCT_NAME} will create a separate Wine prefix for each plugin that runs under Wine. Please note that each Wine prefix is quite large - around 300MB when
					initialised.
				</Tooltip>
			</div>
		{/if}

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-developer" class="text-neutral-400">Enable developer mode:</label>
			<input type="checkbox" bind:checked={$settings.developer} id="settings-developer" />
			<Tooltip>
				This option enables features that make plugin development and debugging easier. Additionally, this option exposes all file paths on your device on the local webserver to allow symbolic linking
				of plugins, so you should disable it if it is not in use.
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-disableelgato" class="text-neutral-400">Disable Elgato device discovery:</label>
			<input type="checkbox" bind:checked={$settings.disableelgato} id="settings-disableelgato" />
			<Tooltip> This option disables discovery of Elgato devices so that they can be managed by other software. </Tooltip>
		</div>
	{/if}

	<div class="ml-2">
		<button
			class="mt-2 mb-4 px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
			on:click={() => invoke("open_config_directory")}
		>
			Open config directory
		</button>
		<button
			class="mt-2 mb-4 px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
			on:click={() => invoke("open_log_directory")}
		>
			Open log directory
		</button>
		<span class="text-xs text-neutral-400">
			{@html buildInfo}
		</span>
		<div class="absolute bottom-6 flex flex-row items-center text-sm text-neutral-400">
			<span class="mr-1">
				Please leave a
				<button on:click={() => invoke("open_url", { url: "https://github.com/nekename/OpenDeck" })} class="underline">star on GitHub</button>
			</span>
			<Star weight="fill" fill="yellow" />
			<span class="mx-1">
				or
				<button on:click={() => invoke("open_url", { url: "https://github.com/sponsors/nekename" })} class="underline">sponsor me</button>
			</span>
			<Heart weight="fill" fill="fuchsia" />
			<span class="ml-1">for my work :)</span>
		</div>
	</div>
</Popup>
