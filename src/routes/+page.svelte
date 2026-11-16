<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import { onMount } from "svelte";

	import { t } from "$lib/i18n";
	import { initPortBase } from "$lib/ports";
	import { initClipboard, inspectedParentAction } from "$lib/propertyInspector";
	import { settings } from "$lib/settings";
	import { actionList, deviceSelector, profileManager } from "$lib/singletons";

	import ActionList from "../components/ActionList.svelte";
	import DeviceSelector from "../components/DeviceSelector.svelte";
	import DeviceView from "../components/DeviceView.svelte";
	import NoDevicesDetected from "../components/NoDevicesDetected.svelte";
	import ParentActionView from "../components/ParentActionView.svelte";
	import PluginManager from "../components/PluginManager.svelte";
	import ProfileManager from "../components/ProfileManager.svelte";
	import PropertyInspectorView from "../components/PropertyInspectorView.svelte";
	import SettingsView from "../components/SettingsView.svelte";

	let devices: { [id: string]: DeviceInfo } = {};
	let selectedDevice: string;
	let selectedProfiles: { [id: string]: Profile } = {};

	// The second window shows keys only: no settings, no plugins, no language.
	import { getCurrentWindow } from "@tauri-apps/api/window";

	// The window label decides: "main" is the settings window, "secondary" only
	// shows keys for another device.
	const secondary = getCurrentWindow().label !== "main";

	let windowTitle = "";
	let secondOpen = false;

	initPortBase();

	onMount(() => {
		initClipboard();
		if (!secondary) {
			import("@tauri-apps/api/event").then(({ listen }) =>
				listen<boolean>("second_window", ({ payload }) => (secondOpen = payload))
			);
		}
	});

	$: if (selectedDevice && devices[selectedDevice]) {
		const title = devices[selectedDevice].name;
		if (title !== windowTitle) {
			windowTitle = title;
			import("@tauri-apps/api/window").then(({ getCurrentWindow }) =>
				getCurrentWindow().setTitle(`OpenDeck - ${title}`)
			);
		}
	}

	$: if ($settings && $settings.second_window === false) {
		import("@tauri-apps/api/core").then(({ invoke }) => invoke("close_second_window"));
	}
</script>

<svelte:window on:dragover={(event) => event.preventDefault()} on:drop={(event) => event.preventDefault()} />

<div class="flex flex-row h-screen">
	<div class="flex flex-col grow min-w-0">
		<nav class="flex flex-row justify-between items-center p-3" class:hidden={$inspectedParentAction}>
			<div class="flex flex-col items-start space-y-1">
				<DeviceSelector bind:devices bind:value={selectedDevice} bind:selectedProfiles bind:this={$deviceSelector} />
				{#key selectedDevice}
					{#if selectedDevice && devices[selectedDevice]}
						<ProfileManager bind:device={devices[selectedDevice]} bind:profile={selectedProfiles[selectedDevice]} bind:this={$profileManager} />
					{/if}
				{/key}
			</div>

			{#if !secondary}
				<div class="flex flex-row items-center space-x-2" class:mr-4={Object.keys(devices).length > 0}>
					{#if $settings?.second_window}
						<button
							class="flex flex-row items-center px-2 py-1 text-sm transition-colors border rounded-lg"
							class:bg-blue-600={secondOpen}
							class:text-white={secondOpen}
							class:hover:bg-blue-500={secondOpen}
							class:border-blue-400={secondOpen}
							class:bg-neutral-700={!secondOpen}
							class:text-neutral-300={!secondOpen}
							class:hover:bg-neutral-600={!secondOpen}
							class:border-neutral-600={!secondOpen}
							on:click={() => import("@tauri-apps/api/core").then(({ invoke }) => invoke("toggle_second_window"))}
						>
							{$t("window.second")}: {secondOpen ? $t("window.second.on") : $t("window.second.off")}
						</button>
					{/if}
					<PluginManager />
					<SettingsView />
				</div>
			{/if}
		</nav>

		{#if Object.keys(devices).length > 0 && selectedProfiles}
			{#if $inspectedParentAction}
				<ParentActionView bind:profile={selectedProfiles[selectedDevice]} />
			{/if}

			{#each Object.entries(devices) as [id, device]}
				{#if device && selectedProfiles[id]}
					<DeviceView bind:device bind:profile={selectedProfiles[id]} bind:selectedDevice />
				{/if}
			{/each}

			{#if selectedProfiles[selectedDevice]}
				<PropertyInspectorView bind:device={devices[selectedDevice]} bind:profile={selectedProfiles[selectedDevice]} />
			{/if}
		{:else}
			<NoDevicesDetected />
		{/if}
	</div>

	{#if !secondary}
		<ActionList bind:this={$actionList} />
	{/if}
</div>
