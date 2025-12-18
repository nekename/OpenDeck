<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import { initPortBase } from "$lib/ports";
	import { inspectedParentAction } from "$lib/propertyInspector";
	import { actionList, deviceSelector, profileManager } from "$lib/singletons";

	import ActionList from "../components/ActionList.svelte";
	import DeviceSelector from "../components/DeviceSelector.svelte";
	import DeviceView from "../components/DeviceView.svelte";
	import NoDevicesDetected from "../components/NoDevicesDetected.svelte";
	import ParentActionView from "../components/ParentActionView.svelte";
	import PluginManager from "../components/PluginManager.svelte";
	import IconPackManager from "../components/IconPacks/IconPackManager.svelte";
	import ProfileManager from "../components/ProfileManager.svelte";
	import PropertyInspectorView from "../components/PropertyInspectorView.svelte";
	import SettingsView from "../components/SettingsView.svelte";

	let devices: { [id: string]: DeviceInfo } = {};
	let selectedDevice: string;
	let selectedProfiles: { [id: string]: Profile } = {};

	initPortBase();
</script>

<svelte:window on:dragover={(event) => event.preventDefault()} on:drop={(event) => event.preventDefault()} />

<div class="flex flex-col grow">
	{#if Object.keys(devices).length > 0 && selectedProfiles}
		{#if $inspectedParentAction}
			<ParentActionView bind:profile={selectedProfiles[selectedDevice]} />
		{/if}

		{#each Object.entries(devices) as [id, device]}
			{#if device && selectedProfiles[id]}
				<DeviceView bind:device bind:profile={selectedProfiles[id]} bind:selectedDevice />
			{/if}
		{/each}
	{:else}
		<NoDevicesDetected />
	{/if}

	{#if selectedProfiles[selectedDevice]}
		<PropertyInspectorView bind:device={devices[selectedDevice]} bind:profile={selectedProfiles[selectedDevice]} />
	{/if}
</div>

<div class="flex flex-col p-2 w-[18rem] h-full border-l dark:border-neutral-700">
	<div class="flex flex-row space-x-2">
		<PluginManager />
		<IconPackManager />
		<SettingsView />
	</div>
	<hr class="my-2 border dark:border-neutral-700" />
	{#if !$inspectedParentAction}
		<DeviceSelector
			bind:devices
			bind:value={selectedDevice}
			bind:selectedProfiles
			bind:this={$deviceSelector}
		/>
		{#key selectedDevice}
			{#if selectedDevice && devices[selectedDevice]}
				<ProfileManager
					bind:device={devices[selectedDevice]}
					bind:profile={selectedProfiles[selectedDevice]}
					bind:this={$profileManager}
				/>
			{/if}
		{/key}
	{/if}
	<ActionList bind:this={$actionList} />
</div>
