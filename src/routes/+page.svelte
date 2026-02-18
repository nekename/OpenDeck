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
	import ProfileManager from "../components/ProfileManager.svelte";
	import PropertyInspectorView from "../components/PropertyInspectorView.svelte";
	import SettingsView from "../components/SettingsView.svelte";

	let devices: { [id: string]: DeviceInfo } = {};
	let selectedDevice: string;
	let selectedProfiles: { [id: string]: Profile } = {};

	initPortBase();
</script>

<svelte:window on:dragover={(event) => event.preventDefault()} on:drop={(event) => event.preventDefault()} />

<div class="flex flex-row h-screen">
	<div class="flex flex-col grow">
		<nav class="flex flex-row justify-between items-center px-3 py-2" class:hidden={$inspectedParentAction}>
			<div class="flex flex-col items-start space-y-1">
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
			</div>

			<div class="flex flex-row items-center space-x-2">
				<PluginManager />
				<SettingsView />
			</div>
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

	<ActionList bind:this={$actionList} />
</div>
