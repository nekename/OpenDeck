<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Profile } from "$lib/Profile";

	import Trash from "phosphor-svelte/lib/Trash";
	import Key from "./Key.svelte";

	import { inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";

	export let profile: Profile;

	let children: ActionInstance[];
	$: children = profile.keys[$inspectedParentAction!.position]!.children!;
	let parentUuid: string;
	$: parentUuid = profile.keys[$inspectedParentAction!.position]!.action.uuid;

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		if (event.dataTransfer?.types.includes("action")) event.dataTransfer.dropEffect = "copy";
	}

	async function handleDrop({ dataTransfer }: DragEvent) {
		if (dataTransfer?.getData("action")) {
			let action = JSON.parse(dataTransfer?.getData("action"));
			if (
				(parentUuid == "opendeck.multiaction" && !action.supported_in_multi_actions) ||
				(
					parentUuid == "opendeck.toggleaction" &&
					(action.uuid == "opendeck.multiaction" || action.uuid == "opendeck.toggleaction")
				)
			) {
				return;
			}
			let response: ActionInstance | null = await invoke("create_instance", { context: $inspectedParentAction, action });
			if (response) profile.keys[$inspectedParentAction!.position]!.children = [...children, response];
		}
	}

	async function removeInstance(index: number) {
		await invoke("remove_instance", { context: children[index].context });
		children.splice(index, 1);
		profile.keys[$inspectedParentAction!.position]!.children = children;
	}
</script>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") $inspectedParentAction = null;
	}}
/>

<div class="px-6 pt-6 pb-4 dark:text-neutral-300">
	<button class="float-right text-xl" on:click={() => $inspectedParentAction = null}>✕</button>
	<h1 class="font-semibold text-2xl">{parentUuid == "opendeck.toggleaction" ? "Toggle Action" : "Multi Action"}</h1>
</div>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class="flex flex-col h-80 overflow-auto"
	on:click={() => inspectedInstance.set(null)}
	on:keyup={() => inspectedInstance.set(null)}
>
	{#each children as instance, index}
		<div class="flex flex-row items-center mx-4 my-1 bg-neutral-100 dark:bg-neutral-800 rounded-md">
			<Key inslot={instance} context={null} active={false} scale={3 / 4} />
			<p class="ml-4 text-xl dark:text-neutral-400">{instance.action.name}</p>
			<button
				class="ml-auto mr-10"
				on:click={() => removeInstance(index)}
			>
				<Trash size="32" class="text-neutral-500 dark:text-neutral-400" />
			</button>
		</div>
	{/each}
	<div
		class="flex flex-row items-center mx-4 mt-1 mb-4 p-3 bg-neutral-100 dark:bg-neutral-800 border-2 border-dashed dark:border-neutral-700 rounded-md"
		on:dragover={handleDragOver}
		on:drop={handleDrop}
	>
		<img src="/cube.png" class="m-2 w-24 rounded-xl" alt="Add new action" />
		<p class="ml-4 text-xl text-neutral-500">Drop actions here</p>
	</div>
</div>
