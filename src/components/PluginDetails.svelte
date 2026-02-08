<script lang="ts">
	import ArrowSquareOut from "phosphor-svelte/lib/ArrowSquareOut";
	import DownloadSimple from "phosphor-svelte/lib/DownloadSimple";
	import Popup from "./Popup.svelte";

	import "$lib/shims.ts";

	import { invoke } from "@tauri-apps/api/core";
	import DOMPurify from "dompurify";
	import { marked } from "marked";
	import { baseUrl } from "marked-base-url";
	import { onMount } from "svelte";

	export let id: string;
	export let details: { repository: string; name: string; author: string; download_url: string | undefined };
	let readme = "<strong>Loading plugin details...</strong>";
	let downloadCount = 0;

	export let install: () => void;
	export let close: () => void;

	// @ts-expect-error
	const fetch = window.fetchNative ?? window.fetch;

	async function getReadme(repo: string): Promise<string> {
		const renderer = new marked.Renderer();
		renderer.link = function (token) {
			const rendered = marked.Renderer.prototype.link.call(this, token);
			return rendered.replace("<a", `<a target="_blank" `);
		};
		marked.use({ renderer });
		const urls = [
			"https://raw.githubusercontent.com/" + repo + "/main/README.md",
			"https://raw.githubusercontent.com/" + repo + "/main/readme.md",
			"https://raw.githubusercontent.com/" + repo + "/master/README.md",
			"https://raw.githubusercontent.com/" + repo + "/master/readme.md",
		];
		for (const url of urls) {
			const response = await fetch(url);
			if (response.ok) {
				marked.use(baseUrl(url));
				return await marked.parse(DOMPurify.sanitize(await response.text()).replace(/<a/g, '<a target="_blank" '));
			}
		}
		return await marked.parse("**Plugin README file not found**\n\n[View plugin on GitHub](https://github.com/" + repo + ")");
	}

	function handleReadmeClick(event: MouseEvent | KeyboardEvent) {
		const link = (event.target as HTMLElement).closest("a");
		if (link && link.href) {
			event.preventDefault();
			window.open(link.href);
		}
	}

	onMount(async () => {
		const repo = details.repository.split("/")[3] + "/" + details.repository.split("/")[4];

		readme = await getReadme(repo);

		const releasesResponse = await fetch("https://api.github.com/repos/" + repo + "/releases");
		const releases = await releasesResponse.json();
		for (const release of releases) {
			for (const asset of release.assets) {
				downloadCount += asset.download_count;
			}
		}
	});
</script>

<Popup show>
	<button class="mr-2 my-1 float-right text-xl dark:text-neutral-300" on:click={close}>✕</button>
	<div class="flex flex-row items-start">
		<img
			src={"https://openactionapi.github.io/plugins/icons/" + id + ".png"}
			alt={details.name}
			class="size-48 rounded-2xl"
		/>
		<div class="flex flex-col justify-center h-48 ml-8">
			<div class="text-3xl dark:text-neutral-200">{details.name}</div>
			<div class="flex items-center mt-2 text-lg text-neutral-600 dark:text-neutral-400">
				<span class="mr-2">by</span>
				<img
					src={"https://avatars.githubusercontent.com/" + details.repository.split("/")[3]}
					alt="Author avatar"
					class="size-7 mr-1.5 rounded-full"
				/>
				<a
					target="_blank"
					href={"https://github.com/" + details.repository.split("/")[3]}
					on:click={() => window.open("https://github.com/" + details.repository.split("/")[3])}
					class="underline"
				>
					{details.author}
					{#if details.repository.split("/")[3] != details.author}
						({details.repository.split("/")[3]})
					{/if}
				</a>
			</div>

			<div class="flex flex-row items-center mt-6">
				<button
					on:click={install}
					class="px-8 py-3 active:translate-y-0.5 text-lg text-neutral-100 bg-indigo-600 rounded-l-lg"
				>
					Install
				</button>

				<button
					on:click={() => invoke("open_url", { url: details.download_url ?? details.repository + "/releases/latest" })}
					class="ml-1 p-3.5 active:translate-y-0.5 text-lg text-neutral-100 bg-indigo-600 rounded-r-lg"
				>
					<ArrowSquareOut size={24} />
				</button>

				{#if downloadCount}
					<div class="flex flex-row ml-6 text-neutral-700 dark:text-neutral-300">
						<span class="mr-1 text-lg">{downloadCount}</span>
						<DownloadSimple size={28} />
					</div>
				{/if}
			</div>
		</div>
	</div>

	<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
	<div
		class="mt-4 p-6 plugin-readme text-neutral-700 dark:text-neutral-300 border-4 border-neutral-300 dark:border-neutral-600 rounded-xl"
		on:click={handleReadmeClick}
		on:keyup={handleReadmeClick}
		role="region"
	>
		{@html readme}
	</div>
</Popup>
