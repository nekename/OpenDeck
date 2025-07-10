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
	let readme = "<h2>Loading plugin details...</h2>";
	let downloadCount = 0;

	export let install: () => void;
	export let close: () => void;

	// @ts-expect-error
	const fetch = window.fetchNative ?? window.fetch;

	onMount(async () => {
		const repo = details.repository.split("/")[3] + "/" + details.repository.split("/")[4];

		const readmeResponse = await fetch("https://raw.githubusercontent.com/" + repo + "/master/README.md");
		marked.use(baseUrl("https://raw.githubusercontent.com/" + repo + "/master/"));
		const renderer = new marked.Renderer();
		renderer.link = function (token) {
			const rendered = marked.Renderer.prototype.link.call(this, token);
			return rendered.replace("<a", `<a target="_blank" onclick="window.open('${token.href}')" `);
		};
		marked.use({ renderer });
		readme = await marked.parse(DOMPurify.sanitize(await readmeResponse.text()).replace(/<a/g, '<a target="_blank" '));

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
	<button class="mr-2 my-1 float-right w-6 h-6 flex items-center justify-center rounded-full text-xl dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-600" on:click={close}>✕</button>
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
					href={"https://github.com/" + details.repository.split("/")[3]}
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
					class="px-8 py-3 active:translate-y-0.5 text-lg text-neutral-100 bg-indigo-600 rounded-l-lg hover:bg-indigo-700"
				>
					Install
				</button>

				<button
					on:click={() => invoke("open_url", { url: details.download_url ?? details.repository + "/releases/latest" })}
					class="ml-1 p-3.5 active:translate-y-0.5 text-lg text-neutral-100 bg-indigo-600 rounded-r-lg hover:bg-indigo-700"
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

	<div class="mt-4 p-6 plugin-readme text-neutral-700 dark:text-neutral-300 border-4 border-neutral-300 dark:border-neutral-600 rounded-xl">
		{@html readme}
	</div>
</Popup>
