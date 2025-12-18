<script lang="ts">
	import { ask, message, open } from "@tauri-apps/plugin-dialog";
	import { FileArrowUp } from "phosphor-svelte";
	import Popup from "../Popup.svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { getWebserverUrl } from "$lib/ports";
	import type { IconPack } from "$lib/IconPacks";
	import { writable, derived } from "svelte/store";
	import { iconPacks } from "./IconPacksStore";
	import InstallPreviewModal from "./InstallPreviewModal.svelte";
	import type { PreviewPackInfo } from "./types";
	import IconPackPreview from "./IconPackPreview.svelte";

	let showPopup: boolean = false;

	$: if (showPopup) {
		if (!$iconPacks.data) {
			iconPacks.fetch();
		}
	}

	const previewObject = writable<PreviewPackInfo | undefined>(undefined);
	const isPreviewInstalled = derived(
		[previewObject, iconPacks],
		([$previewObject, $iconPacks]) =>
			$previewObject
				? $iconPacks.data?.some(
						(pack) => pack.id === $previewObject?.meta.id,
					)
				: false,
	);

	async function previewSDIconPack() {
		const path = await open({
			multiple: false,
			directory: false,
			filters: [
				{
					name: "StreamDeck Icons",
					extensions: ["streamDeckIconPack"],
				},
			],
		});
		if (!path) return;

		await invoke("preview_sd_iconpack", { path }).then((data) => {
			previewObject.set({ path, meta: data as IconPack });
		});
	}
</script>

<button
	class="p-1 w-1/2 text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border dark:border-neutral-600 rounded-lg outline-hidden"
	on:click={() => (showPopup = true)}
>
	Icons
</button>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") {
			showPopup = false;
		}
	}}
/>

<Popup show={showPopup}>
	<button
		class="mr-2 my-1 float-right text-xl dark:text-neutral-300"
		on:click={() => (showPopup = false)}>✕</button
	>
	<h2 class="m-2 font-semibold text-xl dark:text-neutral-300">
		Manage Icon Packs
	</h2>

	<div class="flex flex-row justify-between items-center">
		<h2 class="mx-2 mt-6 mb-2 text-lg dark:text-neutral-400">
			Installed Icon Packs
		</h2>
		<button
			class="flex flex-row items-center mt-2 px-1 py-0.5 text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border dark:border-neutral-600 rounded-lg outline-hidden"
			on:click={previewSDIconPack}
		>
			<FileArrowUp />
			<span class="ml-1">Install from file</span>
		</button>
	</div>

	<div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
		{#if $iconPacks.loading}
			<p>Loading installed icon packs...</p>
		{:else if $iconPacks.error}
			<p class="m-2 text-red-600 dark:text-red-400 col-span-full">
				Error loading icon packs: {$iconPacks.error}
			</p>
		{:else if !$iconPacks.data}
			<p class="m-2 text-neutral-500 dark:text-neutral-400 col-span-full">
				No icon packs installed.
			</p>
		{:else if $iconPacks.data}
			{#each $iconPacks.data as pack}
				<IconPackPreview
					{pack}
					installed={pack.installed_path !== null}
					onRemove={async () => {
						await invoke("uninstall_iconpack", { id: pack.id });
						await iconPacks.dropById(pack.id);
					}}
				/>
			{/each}
		{/if}
		<!--
		{#each installed.sort((a, b) => (a.builtin && !b.builtin) ? -1 : (b.builtin && !a.builtin) ? 1 : a.id.localeCompare(b.id)) as plugin}
			<ListedPlugin
				icon={getWebserverUrl(plugin.icon)}
				name={($localisations && $localisations[plugin.id] && $localisations[plugin.id].Name) ? $localisations[plugin.id].Name : plugin.name}
				subtitle={plugin.version}
				disconnected={!plugin.registered}
				action={() => {
					if ($settings?.developer) invoke("reload_plugin", { id: plugin.id });
					else removePlugin(plugin);
				}}
				secondaryAction={() => {
					if (!plugin.registered) invoke("open_log_directory");
					else if (plugin.has_settings_interface) invoke("show_settings_interface", { plugin: plugin.id });
				}}
			>
				<svelte:fragment slot="subtitle">
					{plugin.version}
					{#if availableUpdates[plugin.id]}
						(<span class="text-yellow-600 dark:text-yellow-400">
							available:
							<button
								class="font-semibold hover:underline outline-hidden"
								on:click={() => openDetailsView = plugin.id.endsWith(".sdPlugin") ? plugin.id.slice(0, -9) : plugin.id}
							>
								{availableUpdates[plugin.id]}
							</button></span>)
					{/if}
				</svelte:fragment>

				<svelte:fragment slot="secondary">
					{#if !plugin.registered}
						<WarningCircle size="24" class="text-yellow-500" />
					{:else if plugin.has_settings_interface}
						<Gear size="24" class="text-green-600" />
					{/if}
				</svelte:fragment>

				{#if $settings?.developer}
					<ArrowClockwise size="24" class="mt-2 text-neutral-500 dark:text-neutral-400" />
				{:else if !plugin.builtin}
					<Trash size="24" class="mt-2 text-neutral-500 dark:text-neutral-400" />
				{/if}
			</ListedPlugin>
		{/each}
		-->
	</div>
</Popup>

<InstallPreviewModal
	open={$previewObject !== undefined}
	pack={$previewObject}
	isInstalled={$isPreviewInstalled}
	onInstall={async (path) => {
		await iconPacks.fetch();
		previewObject.set(undefined);
	}}
/>
