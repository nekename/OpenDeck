<script lang="ts">
	import type { Action } from "$lib/Action";
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Profile } from "$lib/Profile";

	import Trash from "phosphor-svelte/lib/Trash";
	import Key from "./Key.svelte";

	import { copiedItem, inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";
	import { settings } from "$lib/settings";

	import { invoke } from "@tauri-apps/api/core";
	import { onMount, tick } from "svelte";

	export let profile: Profile;

	let listEl: HTMLDivElement;
	onMount(() => {
		const first = listEl?.querySelector("[role='listitem']") as HTMLElement | null;
		first?.focus();
	});

	let children: ActionInstance[];
	$: children = profile.keys[$inspectedParentAction!.position]!.children!;
	let parentUuid: string;
	$: parentUuid = profile.keys[$inspectedParentAction!.position]!.action.uuid;

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		if (event.dataTransfer?.types.includes("action")) event.dataTransfer.dropEffect = "copy";
	}

	async function addAction(action: Action) {
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
		if (response) profile.keys[$inspectedParentAction!.position] = response;
	}

	async function handleDrop({ dataTransfer }: DragEvent) {
		if (dataTransfer?.getData("action")) {
			let action = JSON.parse(dataTransfer?.getData("action"));
			await addAction(action);
		}
	}

	async function handlePaste() {
		if (!$copiedItem || $copiedItem.type != "action") return;
		await addAction($copiedItem.action);
	}

	async function removeInstance(index: number, refocus = false) {
		await invoke("remove_instance", { context: children[index].context });
		children.splice(index, 1);
		profile.keys[$inspectedParentAction!.position]!.children = children;

		if (!refocus) return;

		await tick();
		const items = Array.from(listEl?.querySelectorAll("[role='listitem']") ?? []) as HTMLElement[];
		if (items.length == 0) return;

		const targetIndex = children.length == 0 ? 0 : Math.min(index, children.length - 1);
		for (let i = 0; i < items.length; i++) {
			items[i].tabIndex = i == targetIndex ? 0 : -1;
		}
		items[targetIndex]?.focus();
	}

	function handleListKeydown(event: KeyboardEvent) {
		if (!["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
		const list = event.currentTarget as HTMLElement;
		const items = Array.from(list.querySelectorAll("[role='listitem']"));
		const currentIndex = items.indexOf(document.activeElement?.closest("[role='listitem']") as Element);
		if (currentIndex == -1) return;

		event.preventDefault();

		let newIndex = currentIndex;
		switch (event.key) {
			case "ArrowDown":
				newIndex = Math.min(currentIndex + 1, items.length - 1);
				break;
			case "ArrowUp":
				newIndex = Math.max(currentIndex - 1, 0);
				break;
			case "Home":
				newIndex = 0;
				break;
			case "End":
				newIndex = items.length - 1;
				break;
		}

		if (newIndex == currentIndex) return;
		(items[currentIndex] as HTMLElement).tabIndex = -1;
		(items[newIndex] as HTMLElement).tabIndex = 0;
		(items[newIndex] as HTMLElement).focus();
	}
</script>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") $inspectedParentAction = null;
	}}
/>

<div class="px-6 pt-6 pb-4 text-neutral-300">
	<button class="float-right text-xl" on:click={() => $inspectedParentAction = null} aria-label="Close">✕</button>
	<h1 class="font-semibold text-2xl">{parentUuid == "opendeck.toggleaction" ? "Toggle Action" : "Multi Action"}</h1>
</div>

<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
<div
	bind:this={listEl}
	class="flex flex-col h-128 overflow-auto"
	on:click={() => $inspectedInstance = null}
	role="list"
	aria-label="{parentUuid == 'opendeck.toggleaction' ? 'Toggle Action' : 'Multi Action'} children"
	on:keydown={handleListKeydown}
>
	{#each children as instance, index}
		<!-- svelte-ignore a11y-no-noninteractive-tabindex a11y-no-noninteractive-element-interactions -->
		<div
			class="flex flex-row items-center mx-4 my-2 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg focus-within:outline-solid focus-within:outline-offset-2 focus-within:outline-blue-500"
			on:click|stopPropagation={() => $inspectedInstance = instance.context}
			on:focus|stopPropagation={() => $inspectedInstance = instance.context}
			on:keydown={(e) => {
				if (e.key == "Delete") removeInstance(index, true);
			}}
			role="listitem"
			tabindex={index == 0 ? 0 : -1}
		>
			<div style={`transform: rotate(${-($settings?.device_rotation ?? 0)}deg); transform-origin: center;`}>
				<Key
					inslot={instance}
					context={null}
					active={false}
					scale={3 / 4}
					role="presentation"
					tabindex={-1}
					label={(parentUuid == "opendeck.toggleaction" ? "Toggle Action" : "Multi Action") + " action " + (index + 1)}
				/>
			</div>
			<p class="ml-4 text-xl text-neutral-300">{instance.action.name}</p>
			<button
				class="ml-auto mr-10"
				on:click|stopPropagation={() => removeInstance(index)}
				tabindex={-1}
				aria-label="Remove {instance.action.name}"
			>
				<Trash size="32" class="text-neutral-400" />
			</button>
		</div>
	{/each}
	<!-- svelte-ignore a11y-no-noninteractive-tabindex a11y-no-noninteractive-element-interactions -->
	<div
		class="flex flex-row items-center mx-4 mt-2 mb-4 p-3 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-dashed border-neutral-600 rounded-lg focus-within:outline-solid focus-within:outline-offset-2 focus-within:outline-blue-500"
		on:dragover={handleDragOver}
		on:drop={handleDrop}
		on:click={() => $inspectedInstance = null}
		on:focus={() => $inspectedInstance = null}
		on:keydown={(e) => {
			if ((e.ctrlKey || e.metaKey) && e.key == "v") handlePaste();
		}}
		role="listitem"
		tabindex={children.length == 0 ? 0 : -1}
		aria-label="Drag a new action here or copy one with Control+C and paste with Control+V."
	>
		<img src="/cube.png" class="m-2 w-24 rounded-xl" alt="" />
		<p class="ml-4 text-xl text-neutral-400">Drop or paste actions here</p>
	</div>
</div>
