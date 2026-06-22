<script lang="ts">
	import ClockClockwise from "phosphor-svelte/lib/ClockClockwise";
	import ClockCounterClockwise from "phosphor-svelte/lib/ClockCounterClockwise";
	import Gear from "phosphor-svelte/lib/Gear";
	import Heart from "phosphor-svelte/lib/Heart";
	import Scroll from "phosphor-svelte/lib/Scroll";
	import Star from "phosphor-svelte/lib/Star";
	import Popup from "./Popup.svelte";
	import Tooltip from "./Tooltip.svelte";

	import { settings } from "$lib/settings";
	import { PRODUCT_NAME } from "$lib/singletons";
	import { t } from "$lib/i18n";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { message } from "@tauri-apps/plugin-dialog";

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

	async function backupConfig() {
		await message(
			$t("settings.backupconfig.prompt"),
			{ title: $t("settings.backupconfig.title"), buttons: { ok: $t("dialog.ok") } },
		);
		if (await invoke("backup_config_directory")) {
			await message(
				$t("settings.backupconfig.success.prompt"),
				{ title: $t("settings.backupconfig.success.title"), buttons: { ok: $t("dialog.ok") } },
			);
		}
	}

	async function restoreConfig() {
		await message(
			$t("settings.restoreconfig.prompt"),
			{ title: $t("settings.restoreconfig.title"), buttons: { ok: $t("dialog.ok") } },
		);
		await invoke("restore_config_directory");
	}
</script>

<button
	class="px-3 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
	on:click={() => showPopup = true}
>
	{$t("settings.button")}
</button>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") showPopup = false;
	}}
/>

<Popup show={showPopup} label={$t("settings.button")}>
	<button class="mr-2 my-1 float-right text-xl text-neutral-300" on:click={() => showPopup = false} aria-label={$t("settings.close")}>✕</button>
	<h2 class="m-2 font-semibold text-xl text-neutral-300">{$t("settings.button")}</h2>
	{#if $settings}
		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-language" class="text-neutral-400">{$t("settings.language")}</label>
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
				{$t("settings.language.tooltip").replaceAll("{PRODUCT_NAME}", PRODUCT_NAME)}
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-brightness" class="text-neutral-400">{$t("settings.device.brightness")}</label>
			<input type="range" min="0" max="100" bind:value={$settings.brightness} id="settings-brightness" />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-sleep_timeout_minutes" class="text-neutral-400">{$t("settings.device.sleep")}</label>
			<input type="number" min="0" bind:value={$settings.sleep_timeout_minutes} class="w-12 px-1 text-neutral-300 border border-neutral-600 rounded-lg" id="settings-sleep_timeout_minutes" />
			<span class="text-neutral-400">{$t("settings.time.minutes")}</span>
			<Tooltip> {$t("settings.device.sleep.tooltip")} </Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-sleep_when_computer_locked" class="text-neutral-400">{$t("settings.device.sleep_on_lock")}</label>
			<input type="checkbox" bind:checked={$settings.sleep_when_computer_locked} id="settings-sleep_when_computer_locked" />
			<Tooltip> {$t("settings.device.sleep_on_lock.tooltip")} </Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-rotation" class="text-neutral-400">{$t("settings.device.image.rotation")}</label>
			<input type="range" min="0" max="270" step="90" bind:value={$settings.rotation} id="settings-rotation" />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-background" class="text-neutral-400">{$t("settings.background")}</label>
			<input type="checkbox" bind:checked={$settings.background} id="settings-background" />
			<Tooltip>{$t("settings.background.tooltip").replaceAll("{PRODUCT_NAME}", PRODUCT_NAME)}</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-autolaunch" class="text-neutral-400">{$t("settings.startlogin")}</label>
			<input type="checkbox" bind:checked={$settings.autolaunch} id="settings-autolaunch" />
			<Tooltip>
				{$t("settings.startlogin1.tooltip").replaceAll("{PRODUCT_NAME}", PRODUCT_NAME)}
				{#if buildInfo?.split("</summary>")[0]?.includes("linux")}
					<br />
					{$t("settings.startlogin2.tooltip").replaceAll("{PRODUCT_NAME}", PRODUCT_NAME)}
				{/if}
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-updatecheck" class="text-neutral-400">{$t("settings.updates")}</label>
			<input type="checkbox" bind:checked={$settings.updatecheck} id="settings-updatecheck" />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-statistics" class="text-neutral-400">{$t("settings.statistic")}</label>
			<input type="checkbox" bind:checked={$settings.statistics} id="settings-statistics" />
		</div>

		{#if !buildInfo?.split("</summary>")[0]?.includes("windows")}
			<div class="flex flex-row items-center m-2 space-x-2">
				<label for="settings-separatewine" class="text-neutral-400">{$t("settings.wine")}</label>
				<input type="checkbox" bind:checked={$settings.separatewine} id="settings-separatewine" />
				<Tooltip>
					{$t("settings.wine.tooltip").replaceAll("{PRODUCT_NAME}", PRODUCT_NAME)}
				</Tooltip>
			</div>
		{/if}

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-developer" class="text-neutral-400">{$t("settings.devmode")}</label>
			<input type="checkbox" bind:checked={$settings.developer} id="settings-developer" />
			<Tooltip>
				{$t("settings.devmode.tooltip")}
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-disableelgato" class="text-neutral-400">{$t("settings.disableelgato")}</label>
			<input type="checkbox" bind:checked={$settings.disableelgato} id="settings-disableelgato" />
			<Tooltip>{$t("settings.disableelgato.tooltip")}</Tooltip>
		</div>
	{/if}

	<div class="ml-2">
		<div class="flex flex-row my-3 space-x-2">
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => backupConfig()}
			>
				<ClockCounterClockwise class="mr-1" />
				{$t("settings.backupconfig.button")}
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => restoreConfig()}
			>
				<ClockClockwise class="mr-1" />
				{$t("settings.restoreconfig.button")}
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => invoke("open_config_directory")}
			>
				<Gear class="mr-1" />
				{$t("settings.configdir")}
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => invoke("open_log_directory")}
			>
				<Scroll class="mr-1" />
				{$t("settings.logdir")}
			</button>
		</div>

		<span class="text-xs text-neutral-400">
			{@html buildInfo}
		</span>
		<div class="absolute bottom-6 flex flex-row items-center text-sm text-neutral-400">
			<span class="mr-1">
				{$t("settings.foot1")}
				<button on:click={() => invoke("open_url", { url: "https://github.com/nekename/OpenDeck" })} class="underline">{$t("settings.foot2")}</button>
			</span>
			<Star weight="fill" fill="yellow" />
			<span class="mx-1">
				{$t("settings.foot3")}
				<button on:click={() => invoke("open_url", { url: "https://github.com/sponsors/nekename" })} class="underline">{$t("settings.foot4")}</button>
			</span>
			<Heart weight="fill" fill="fuchsia" />
			<span class="ml-1">{$t("settings.foot5")}</span>
		</div>
	</div>
</Popup>
