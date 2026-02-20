<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import { getWebserverUrl, getWebSocketPort } from "$lib/ports";
	import { inspectedInstance } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";

	let iframes: { [context: string]: HTMLIFrameElement } = {};
	let iframeContainer: HTMLDivElement;
	let iframeClosePopup: HTMLButtonElement;
	let iframePopupsOpen: string[] = [];

	export let device: DeviceInfo;
	export let profile: Profile;

	async function iframeOnLoad(instance: ActionInstance) {
		const iframe = iframes[instance.context];
		const split = instance.context.split(".");

		const position = parseInt(split[3]);
		let coordinates: { row: number; column: number };
		if (split[2] == "Encoder") {
			coordinates = { row: 0, column: position };
		} else {
			coordinates = { row: Math.floor(position / device.columns), column: position % device.columns };
		}

		if (instance == null || !iframe.src || !iframe.src.startsWith(getWebserverUrl())) return;
		const info = JSON.stringify(await invoke("make_info", { plugin: instance.action.plugin }));

		iframe?.contentWindow?.postMessage({
			event: "connect",
			payload: [
				getWebSocketPort(),
				instance.context,
				"registerPropertyInspector",
				info,
				JSON.stringify({
					action: instance.action.uuid,
					context: instance.context,
					device: split[0],
					payload: {
						settings: instance.settings,
						coordinates,
						controller: split[2],
						state: instance.current_state,
						isInMultiAction: parseInt(split[4]) != 0,
					},
				}),
			],
		}, getWebserverUrl());
	}

	const closePopup = (context: string) => {
		const iframe = iframes[context];
		if (iframe) {
			iframe.style.position = "";
			iframe.style.left = "";
			iframe.style.top = "";
			iframe.style.width = "100%";
			iframe.style.height = "100%";
			iframe.style.display = $inspectedInstance == context ? "block" : "none";
			iframe.contentWindow?.postMessage({ event: "windowClosed" }, getWebserverUrl());
		}

		iframePopupsOpen = iframePopupsOpen.filter((e) => e != context);

		if (iframePopupsOpen.length == 0) {
			iframeContainer.style.position = "";
			iframeContainer.style.width = "";
			iframeContainer.style.height = "";
			iframeContainer.style.padding = "";
			iframeContainer.style.zIndex = "0";

			iframeClosePopup.style.display = "none";
		}
	};

	window.addEventListener("message", ({ data }) => {
		if (data.event == "windowOpened") {
			const iframe = iframes[data.payload];
			iframe.style.position = "absolute";
			iframe.style.left = "36px";
			iframe.style.top = "36px";
			iframe.style.width = "calc(100% - 72px)";
			iframe.style.height = "calc(100% - 72px)";
			iframe.style.display = "block";

			iframePopupsOpen.push(data.payload);

			iframeContainer.style.position = "absolute";
			iframeContainer.style.width = "100%";
			iframeContainer.style.height = "100%";
			iframeContainer.style.padding = "36px";
			iframeContainer.style.zIndex = "20";

			iframeClosePopup.style.display = "block";
		} else if (data.event == "windowClosed") {
			closePopup(data.payload);
		} else if (data.event == "openUrl") {
			invoke("open_url", { url: data.payload });
		} else if (data.event == "fetch") {
			function combineUint8Arrays(arrays: Uint8Array[]): Uint8Array {
				const totalLength = arrays.reduce((acc, curr) => acc + curr.length, 0);
				let mergedArray = new Uint8Array(totalLength);
				let offset = 0;

				arrays.forEach((item) => {
					mergedArray.set(item, offset);
					offset += item.length;
				});

				return mergedArray;
			}

			// @ts-expect-error
			window.fetchCORS(...data.payload.args).then(async (response: Response) => {
				const chunks = [];
				if (response.body) {
					const reader = response.body.getReader();
					while (true) {
						const { done, value } = await reader.read();
						if (done) break;
						chunks.push(value);
					}
				}
				const body = combineUint8Arrays(chunks);

				iframes[data.payload.context]?.contentWindow?.postMessage({
					event: "fetchResponse",
					payload: {
						id: data.payload.id,
						response: {
							url: response.url,
							body,
							headers: response.headers.entries().toArray(),
							status: response.status,
							statusText: response.statusText,
						},
					},
				}, getWebserverUrl());
			}).catch((error: any) => {
				iframes[data.payload.context]?.contentWindow?.postMessage({ event: "fetchError", payload: { id: data.payload.id, error } }, getWebserverUrl());
			});
		}
	});

	const nonNull = <T>(o: T | null): o is T => o != null;
	$: instances = profile
		.keys.filter(nonNull)
		.reduce((prev, current) => prev.concat(current.children ? [current, ...current.children] : current), [] as ActionInstance[])
		.concat(profile.sliders.filter(nonNull));

	listen("plugin_reloaded", ({ payload }: { payload: string }) => {
		for (const instance of instances) {
			if (instance.action.plugin == payload && iframes[instance.context]) {
				iframes[instance.context].src += "";
				if ($inspectedInstance == instance.context) {
					invoke("switch_property_inspector", { new: instance.context });
				}
			}
		}
	});
</script>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape" && iframePopupsOpen.length > 0) {
			closePopup(iframePopupsOpen[iframePopupsOpen.length - 1]);
		}
	}}
/>

<div class="grow min-h-64 max-h-96 overflow-auto bg-neutral-800 border-t border-neutral-700" bind:this={iframeContainer}>
	<button
		bind:this={iframeClosePopup}
		on:click={() => closePopup(iframePopupsOpen[iframePopupsOpen.length - 1])}
		class="absolute top-2 right-2 text-2xl text-neutral-300 font-bold hidden"
	>
		✕
	</button>
	{#each instances as instance (instance.context)}
		{#if instance.action.property_inspector}
			<iframe
				title="Property inspector"
				class="w-full h-full hidden"
				class:block!={$inspectedInstance == instance.context}
				src={getWebserverUrl(instance.action.property_inspector + "|opendeck_property_inspector")}
				name={instance.context}
				bind:this={iframes[instance.context]}
				on:load={() => iframeOnLoad(instance)}
			/>
		{/if}
	{/each}
</div>
