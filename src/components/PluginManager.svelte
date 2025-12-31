<script lang="ts">
	import ArrowClockwise from "phosphor-svelte/lib/ArrowClockwise";
	import ArrowSquareOut from "phosphor-svelte/lib/ArrowSquareOut";
	import CloudArrowDown from "phosphor-svelte/lib/CloudArrowDown";
	import FileArrowUp from "phosphor-svelte/lib/FileArrowUp";
	import Gear from "phosphor-svelte/lib/Gear";
	import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";
	import Trash from "phosphor-svelte/lib/Trash";
	import WarningCircle from "phosphor-svelte/lib/WarningCircle";
	import ListedPlugin from "./ListedPlugin.svelte";
	import PluginDetails from "./PluginDetails.svelte";
	import Popup from "./Popup.svelte";
	import Tooltip from "./Tooltip.svelte";

	import { getWebserverUrl } from "$lib/ports";
	import { localisations, settings } from "$lib/settings";
	import { actionList, deviceSelector, PRODUCT_NAME } from "$lib/singletons";

	import { invoke } from "@tauri-apps/api/core";
	import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
	import { ask, message, open } from "@tauri-apps/plugin-dialog";

	// @ts-expect-error
	const fetch = window.fetchNative ?? window.fetch;

	let showPopup: boolean;
	setInterval(async () => {
		if (showPopup) installed = await invoke("list_plugins");
	}, 1e3);

	async function installPlugin(name: string, url: string | null, file: string | null, fallback_id: string | null) {
		if (!file && !await ask(`It may take a while to install the plugin.`, { title: `Install "${name}"?` })) return;
		try {
			await invoke("install_plugin", { url, file, fallback_id });
			message(`Successfully installed "${name}".`, { title: `Installed "${name}"` });
			$actionList?.reload();
			installed = await invoke("list_plugins");
		} catch (error: any) {
			message(error, { title: `Failed to install "${name}"` });
		}
	}

	let choices: any[] | undefined;
	let choice: number;
	let finishChoice = (_: unknown) => {};
	let cancelChoice = () => {};
	async function chooseAsset(assets: any[]): Promise<any> {
		choices = assets;
		try {
			await new Promise((resolve, reject) => {
				finishChoice = resolve;
				cancelChoice = reject;
			});
		} catch (e) {
			throw e;
		} finally {
			choices = undefined;
			finishChoice = (_: unknown) => {};
			cancelChoice = () => {};
		}
		return assets[choice];
	}

	let openDetailsView: string | null = null;
	type GitHubPlugin = {
		name: string;
		author: string;
		repository: string;
		download_url: string | undefined;
	};
	async function installPluginGitHub(id: string, plugin: GitHubPlugin) {
		if (plugin.download_url) {
			await installPlugin(plugin.name, plugin.download_url, null, id);
			return;
		}

		let endpoint = new URL(plugin.repository);
		endpoint.hostname = "api." + endpoint.hostname;
		endpoint.pathname = "/repos" + endpoint.pathname + "/releases";

		let res;
		try {
			res = await (await fetch(endpoint)).json();
		} catch (error: any) {
			message(error, { title: `Failed to install "${plugin.name}"` });
			return;
		}

		let assets = [];
		for (const asset of res[0].assets) {
			if (asset.name.toLowerCase().endsWith(".streamdeckplugin") || asset.name.toLowerCase().endsWith(".zip")) {
				assets.push(asset);
			}
		}
		let selected;
		if (assets.length == 1) selected = assets[0];
		else {
			try {
				selected = await chooseAsset(assets);
			} catch {
				return;
			}
		}

		await installPlugin(plugin.name, selected.browser_download_url, null, id);
	}

	async function installPluginElgato(plugin: any) {
		await installPlugin(plugin.name, `https://plugins.amankhanna.me/rezipped/${plugin.id}.zip`, null, plugin.id);
	}

	async function installPluginFile() {
		const path = await open({ multiple: false, directory: false });
		if (!path) return;
		await installPlugin(path.split(/[\/\\]/).at(-1) ?? path, null, path, null);
	}

	async function removePlugin(plugin: any) {
		if (!await ask(`Are you sure you want to remove "${plugin.name}"?`, { title: `Remove "${plugin.name}"?` })) return;
		try {
			await invoke("remove_plugin", { id: plugin.id });
			message(`Successfully removed "${plugin.name}".`, { title: `Removed "${plugin.name}"` });
			$actionList?.reload();
			$deviceSelector?.reloadProfiles();
			installed = await invoke("list_plugins");
		} catch (error: any) {
			message(error, { title: `Failed to remove "${plugin.name}"` });
		}
	}

	async function isUpdateAvailable(plugin: any): Promise<string | false> {
		const id = plugin.id.endsWith(".sdPlugin") ? plugin.id.slice(0, -9) : plugin.id;
		const cataloguePlugin = plugins[id];
		if (!cataloguePlugin || cataloguePlugin.download_url) return false;

		try {
			const endpoint = new URL(cataloguePlugin.repository);
			endpoint.hostname = "api." + endpoint.hostname;
			endpoint.pathname = "/repos" + endpoint.pathname + "/releases/latest";

			const res = await fetch(endpoint);
			if (!res.ok) return false;
			const release = await res.json();

			const normalizeVersion = (v: string) => v.replace(/^v/, "").replace(/^(\d+\.\d+\.\d+)\.\d+$/, "$1");
			if (normalizeVersion(release.tag_name) != normalizeVersion(plugin.version)) {
				return release.tag_name.replace(/^v/, "");
			} else {
				return false;
			}
		} catch (error) {
			console.warn("Failed to check for plugin update:", error);
			return false;
		}
	}

	let installed: any[] = [];
	(async () => installed = await invoke("list_plugins"))();

	let plugins: { [id: string]: GitHubPlugin };
	(async () => plugins = await (await fetch("https://openactionapi.github.io/plugins/catalogue.json")).json())();

	let availableUpdates: { [id: string]: string | false } = {};
	let checkedPlugins = new Set<string>();
	$: if (showPopup) {
		for (const plugin of installed) {
			if (!checkedPlugins.has(plugin.id)) {
				checkedPlugins.add(plugin.id);
				isUpdateAvailable(plugin)
					.then((version) => availableUpdates = { ...availableUpdates, [plugin.id]: version });
			}
		}
	}

	let pluginVersions: { [id: string]: string } = {};
	$: for (const plugin of installed) {
		if (pluginVersions[plugin.id] != plugin.version) {
			checkedPlugins.delete(plugin.id);
			delete availableUpdates[plugin.id];
			availableUpdates = availableUpdates;
			pluginVersions[plugin.id] = plugin.version;
		}
	}

	let query: string = "";

	onOpenUrl((urls: string[]) => {
		if (!urls[0].includes("installPlugin/")) return;
		let id = urls[0].split("installPlugin/")[1];
		if (!plugins[id]) return;
		installPluginGitHub(id, plugins[id]);
	});
</script>

<button
	class="p-1 w-1/2 text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border dark:border-neutral-600 rounded-lg outline-hidden"
	on:click={() => showPopup = true}
>
	Plugins
</button>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") {
			if (choices) cancelChoice();
			else if (openDetailsView) openDetailsView = null;
			else showPopup = false;
		}
	}}
/>

<Popup show={showPopup}>
	<button class="mr-2 my-1 float-right text-xl dark:text-neutral-300" on:click={() => showPopup = false}>✕</button>
	<h2 class="m-2 font-semibold text-xl dark:text-neutral-300">Manage plugins</h2>

	<h2 class="mx-2 mt-6 mb-2 text-lg dark:text-neutral-400">Installed plugins</h2>
	<div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
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
	</div>

	<div class="flex flex-row justify-between items-center mx-2 mt-6 mb-2">
		<h2 class="text-lg dark:text-neutral-400">Plugin store</h2>
		<button
			class="flex flex-row items-center mt-2 px-1 py-0.5 text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border dark:border-neutral-600 rounded-lg outline-hidden"
			on:click={installPluginFile}
		>
			<FileArrowUp />
			<span class="ml-1">Install from file</span>
		</button>
	</div>

	<div class="flex flex-row items-center mx-2 my-4 p-3 space-x-2 bg-yellow-50 dark:bg-yellow-900/20 border-l-4 border-yellow-500 rounded">
		<WarningCircle size="20" class="mt-0.5 text-yellow-500" />
		<div class="text-sm text-yellow-800 dark:text-yellow-200">
			If you are experiencing issues with a plugin, please reach out on one of the {PRODUCT_NAME} support channels before attempting to contact the plugin developer.
		</div>
	</div>

	<div class="flex flex-row items-center m-2 bg-neutral-200 dark:bg-neutral-700 rounded-md">
		<MagnifyingGlass size="14" class="ml-3 mr-0.5 text-neutral-500 dark:text-neutral-300" />
		<input
			bind:value={query}
			class="w-full p-2 dark:text-neutral-300 outline-hidden"
			placeholder="Search plugins"
			type="search"
			spellcheck="false"
		/>
	</div>

	<div class="ml-2 mt-8 mb-4">
		<h2 class="font-semibold text-md dark:text-neutral-400">Elgato Marketplace</h2>
		<button
			on:click={() => invoke("open_url", { url: "https://github.com/nekename/OpenDeck/wiki/0.-Elgato-Marketplace" })}
			class="mt-4 text-md text-blue-400 hover:underline"
		>
			Click here for instructions
		</button>
	</div>

	{#if !plugins}
		<h2 class="mx-2 mt-6 mb-2 text-md dark:text-neutral-400">Loading open-source plugin list...</h2>
	{:else}
		<div class="flex flex-row items-center ml-2 mt-6 mb-2 space-x-2">
			<h2 class="font-semibold text-md dark:text-neutral-400">Open-source plugins</h2>
			<Tooltip>Open-source plugins downloaded from the author's releases</Tooltip>
		</div>
		<div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
			{#each Object.entries(plugins) as [id, plugin]}
				<ListedPlugin
					icon="https://openactionapi.github.io/plugins/icons/{id}.png"
					name={plugin.name}
					subtitle={plugin.author}
					hidden={!plugin.name.toLowerCase().includes(query.toLowerCase())}
					action={() => openDetailsView = id}
				>
					<ArrowSquareOut size="24" class="text-neutral-500 dark:text-neutral-400" />
				</ListedPlugin>
			{/each}
		</div>
	{/if}

	{#await fetch("https://plugins.amankhanna.me/catalogue.json")}
		<h2 class="mx-2 mt-6 mb-2 text-md dark:text-neutral-400">Loading Elgato App Store archive plugin list...</h2>
	{:then archiveRes}
		<div class="flex flex-row items-center mt-6 mb-2">
			<h2 class="mx-2 font-semibold text-md dark:text-neutral-400">Elgato App Store archive</h2>
			<Tooltip>Plugins archived from the Elgato App Store (now replaced by the Elgato Marketplace)</Tooltip>
		</div>
		{#await archiveRes.json() then entries}
			<div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
				{#each entries as plugin}
					<ListedPlugin
						icon="https://plugins.amankhanna.me/icons/{plugin.id}.png"
						name={plugin.name}
						subtitle={plugin.author}
						hidden={!plugin.name.toLowerCase().includes(query.toLowerCase())}
						action={() => installPluginElgato(plugin)}
					>
						<CloudArrowDown size="24" class="text-neutral-500 dark:text-neutral-400" />
					</ListedPlugin>
				{/each}
			</div>
		{/await}
	{/await}

	{#if "Tacto Connect".toLowerCase().includes(query.toLowerCase())}
		<div class="flex flex-row items-center mt-6 mb-2">
			<h2 class="mx-2 font-semibold text-md dark:text-neutral-400">Tacto</h2>
			<Tooltip>Turn your phone or keyboard into a control centre for your computer</Tooltip>
		</div>
		<div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
			<ListedPlugin
				icon="https://tacto.rivul.us/icon-192.png"
				name="Tacto Connect"
				subtitle="Rivulus"
				action={() =>
					installPluginGitHub("us.rivul.tacto", {
						name: "Tacto Connect",
						author: "Rivulus",
						repository: "https://github.com/RivulusLive/tacto-desktop",
						download_url: undefined,
					})}
				secondaryAction={() => window.open("https://tacto.rivul.us")}
			>
				<svelte:fragment slot="secondary">
					<ArrowSquareOut size="24" class="text-neutral-500 dark:text-neutral-400" />
				</svelte:fragment>

				<CloudArrowDown size="24" class="mt-2 text-neutral-500 dark:text-neutral-400" />
			</ListedPlugin>
		</div>
	{/if}
</Popup>

{#if openDetailsView}
	<PluginDetails
		id={openDetailsView}
		details={plugins[openDetailsView]}
		install={() => {
			// @ts-expect-error
			installPluginGitHub(openDetailsView, plugins[openDetailsView]);
		}}
		close={() => openDetailsView = null}
	/>
{/if}

{#if choices}
	<div class="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 mt-2 p-2 w-96 text-xs dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border-2 dark:border-neutral-600 rounded-lg z-40">
		<h3 class="mb-2 font-semibold text-lg text-center">Choose a release asset</h3>
		<div class="select-wrapper">
			<select class="w-full" bind:value={choice}>
				{#each choices as choice, i}
					<option value={i}>{choice.name}</option>
				{/each}
			</select>
		</div>
		<button
			class="mt-2 p-1 w-full text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-200 dark:bg-neutral-800 border dark:border-neutral-600 rounded-lg"
			on:click={finishChoice}
		>
			Install
		</button>
	</div>
{/if}
