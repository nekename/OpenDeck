<script lang="ts">
	import { ask, message, open } from "@tauri-apps/plugin-dialog";
	import { FileArrowUp, MagnifyingGlass } from "phosphor-svelte";
	import { invoke } from "@tauri-apps/api/core";
	import type { IconPack } from "$lib/IconPacks";
	import { writable, derived } from "svelte/store";
	import { iconPacks } from "./IconPacksStore";
	import InstallPreviewModal from "./InstallPreviewModal.svelte";
	import type { PreviewPackInfo } from "./types";
	import IconPackPreview from "./IconPackPreview.svelte";
	import Modal from './Modal.svelte';
	import IconPickerModal from "./IconPickerModal.svelte";

	let showPopup: boolean = false;
	let showIconPreview: boolean = false;

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

<IconPickerModal bind:open={showIconPreview} />

<Modal bind:open={showPopup}>
	<h2 slot="header" class="font-semibold text-xl dark:text-neutral-300">
		Manage Icon Packs
	</h2>

	<div slot="children" class="flex flex-col gap-4 mt-4 mb-4">
		<div class="flex flex-row items-center">
			<button
				class="flex flex-row gap-1 px-2 py-0.5 items-center text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border dark:border-neutral-600 rounded-lg outline-hidden"
				on:click={() => { showIconPreview = true; }}
			>
				<MagnifyingGlass />
				<span>Explore installed icons</span>
			</button>
		</div>

		<div class="flex flex-row justify-between items-center">
			<h2 class="text-lg dark:text-neutral-400">
				Installed Icon Packs
			</h2>
			<button
				class="flex flex-row gap-1 px-2 py-0.5 items-center text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border dark:border-neutral-600 rounded-lg outline-hidden"
				on:click={previewSDIconPack}
			>
				<FileArrowUp />
				<span>Install from file</span>
			</button>
		</div>

		<div class="grid gap-4 grid-cols-2">
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
		</div>
	</div>
</Modal>

<InstallPreviewModal
	open={$previewObject !== undefined}
	pack={$previewObject}
	isInstalled={$isPreviewInstalled}
	onInstall={async (path) => {
		await iconPacks.fetch();
		previewObject.set(undefined);
	}}
/>
