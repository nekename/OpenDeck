<script lang="ts">
	import Heart from "phosphor-svelte/lib/Heart";
	import Star from "phosphor-svelte/lib/Star";
	import Popup from "./Popup.svelte";
	import Tooltip from "./Tooltip.svelte";

	import { settings } from "$lib/settings";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";

	let showPopup: boolean;
	let buildInfo: string;
	(async () => buildInfo = await invoke("get_build_info"))();

	settings.subscribe((settings) => {
		if (settings) updateTheme(settings.darktheme);
	});

	function updateTheme(darktheme: boolean) {
		if (darktheme) {
			document.documentElement.classList.add("dark");
		} else {
			document.documentElement.classList.remove("dark");
		}
	}

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
	class="ml-2 mt-2 p-1 w-1/2 text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 hover:bg-neutral-200 dark:bg-neutral-700 dark:hover:bg-neutral-800 border dark:border-neutral-600 rounded-lg outline-hidden"
	on:click={() => showPopup = true}
>
	Settings
</button>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") showPopup = false;
	}}
/>

<Popup show={showPopup}>
	<button
		class="mr-2 my-1 float-right w-6 h-6 flex items-center justify-center rounded-full text-xl dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-600"
		on:click={() => showPopup = false}
	>
		✕
	</button>
	<h2 class="m-2 font-semibold text-xl dark:text-neutral-300">Settings</h2>
	{#if $settings}
		<div class="flex flex-row items-center m-2 space-x-2">
			<span class="dark:text-neutral-400"> Language: </span>
			<div class="select-wrapper">
				<select bind:value={$settings.language} class="w-32">
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
				OpenDeck itself is not yet translated. Changing this setting will translate the text from installed plugins into your language for those that support it.
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<span class="dark:text-neutral-400"> Device brightness: </span>
			<input type="range" min="0" max="100" bind:value={$settings.brightness} />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<span class="dark:text-neutral-400"> Enable dark theme: </span>
			<input type="checkbox" bind:checked={$settings.darktheme} />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<span class="dark:text-neutral-400"> Run in background: </span>
			<input type="checkbox" bind:checked={$settings.background} />
			<Tooltip> If this option is enabled, OpenDeck will minimise to the tray and run in the background. </Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<span class="dark:text-neutral-400"> Start at login: </span>
			<input type="checkbox" bind:checked={$settings.autolaunch} />
			<Tooltip> If this option is enabled, OpenDeck will automatically start at login. </Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<span class="dark:text-neutral-400"> Check for updates: </span>
			<input type="checkbox" bind:checked={$settings.updatecheck} />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<span class="dark:text-neutral-400"> Contribute statistics: </span>
			<input type="checkbox" bind:checked={$settings.statistics} />
		</div>

		{#if !buildInfo?.includes("windows")}
			<div class="flex flex-row items-center m-2 space-x-2">
				<span class="dark:text-neutral-400"> Create separate Wine prefixes: </span>
				<input type="checkbox" bind:checked={$settings.separatewine} />
				<Tooltip>
					If this option is enabled, OpenDeck will create a separate Wine prefix for each plugin that runs under Wine. Please note that each Wine prefix is quite large - around 300MB when initialised.
				</Tooltip>
			</div>
		{/if}

		<div class="flex flex-row items-center m-2 space-x-2">
			<span class="dark:text-neutral-400"> Enable developer mode: </span>
			<input type="checkbox" bind:checked={$settings.developer} />
			<Tooltip>
				This option enables features that make plugin development and debugging easier. Additionally, this option exposes all file paths on your device on the local webserver to allow symbolic linking
				of plugins, so you should disable it if it is not in use.
			</Tooltip>
		</div>
	{/if}

	<div class="ml-2">
		<button
			class="mt-2 mb-4 px-2 py-1 text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 hover:bg-neutral-200 dark:bg-neutral-700 dark:hover:bg-neutral-800 border dark:border-neutral-600 rounded-lg"
			on:click={() => invoke("open_config_directory")}
		>
			Open config directory
		</button>
		<button
			class="mt-2 mb-4 px-2 py-1 text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 hover:bg-neutral-200 dark:bg-neutral-700 dark:hover:bg-neutral-800 border dark:border-neutral-600 rounded-lg"
			on:click={() => invoke("open_log_directory")}
		>
			Open log directory
		</button>
		<span class="text-xs dark:text-neutral-400">
			{@html buildInfo}
		</span>
		<div class="absolute bottom-6 flex flex-row items-center text-sm dark:text-neutral-400">
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
