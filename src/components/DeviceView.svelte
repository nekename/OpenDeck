<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Context } from "$lib/Context";
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import Key from "./Key.svelte";

	import { inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";

	export let device: DeviceInfo;
	export let profile: Profile;

	export let selectedDevice: string;


	function handleDragStart({ dataTransfer }: DragEvent, controller: string, position: number) {
		if (!dataTransfer) return;
		dataTransfer.effectAllowed = "move";
		dataTransfer.setData("controller", controller);
		dataTransfer.setData("position", position.toString());
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		if (!event.dataTransfer) return;
		if (event.dataTransfer.types.includes("action")) event.dataTransfer.dropEffect = "copy";
		else if (event.dataTransfer.types.includes("controller")) event.dataTransfer.dropEffect = "move";
	}

	async function handleDrop({ dataTransfer }: DragEvent, controller: string, position: number) {
		let context = { device: device.id, profile: profile.id, controller, position };
		let array = controller == "Encoder" ? profile.sliders : controller == "Infobar" ? profile.infobar : profile.keys;
		
		if (dataTransfer?.getData("action")) {
			let action = JSON.parse(dataTransfer?.getData("action"));
			if (array[position]) {
				return;
			}
			console.log("drop context:", context);
			console.log("drop action:", action);
			let response = (await invoke("create_instance", { context, action })) as any;
			console.log("create response:", response);
			array[position] = response;
			profile = profile;
		} else if (dataTransfer?.getData("controller")) {
			let oldController = dataTransfer?.getData("controller");
			let oldArray = oldController == "Encoder" ? profile.sliders : oldController == "Infobar" ? profile.infobar : profile.keys;
			let oldPosition = parseInt(dataTransfer?.getData("position"));
			let response = (await invoke("move_instance", {
				source: { device: device.id, profile: profile.id, controller: oldController, position: oldPosition },
				destination: context,
				retain: false,
			})) as any;
			if (response) {
				array[position] = response;
				oldArray[oldPosition] = null;
				profile = profile;
			}
		}
	}

	async function handlePaste(source: Context, destination: Context) {
		let response = (await invoke("move_instance", { source, destination, retain: true })) as any;
		if (response) {
			let array = destination.controller == "Encoder" ? profile.sliders : destination.controller == "Infobar" ? profile.infobar : profile.keys;
			array[destination.position] = response;
			profile = profile;
		}
	}

	$: overflowsX = Math.max(device.columns, device.encoders, device.touchpoints) > 8;
	$: overflowsY = (device.rows + Math.min(device.encoders, 1) + Math.min(device.touchpoints, 1)) > 4;
</script>

<style>
	.device-fade-x {
		mask-image: linear-gradient(to right, transparent, black 7.5rem, black calc(100% - 7.5rem), transparent);
	}
	.device-fade-y {
		mask-image: linear-gradient(to bottom, transparent, black 7.5rem, black calc(100% - 7.5rem), transparent);
	}
	.device-fade-xy {
		mask-image:
			linear-gradient(to right, transparent, black 7.5rem, black calc(100% - 7.5rem), transparent),
			linear-gradient(to bottom, transparent, black 7.5rem, black calc(100% - 7.5rem), transparent);
		mask-composite: intersect;
	}
</style>

{#key device}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div
		class="flex flex-col justify-center grow px-16 py-6 overflow-auto"
		class:items-center={device.columns <= 8}
		class:hidden={$inspectedParentAction || selectedDevice != device.id}
		class:device-fade-x={overflowsX && !overflowsY}
		class:device-fade-y={overflowsY && !overflowsX}
		class:device-fade-xy={overflowsX && overflowsY}
		on:click={() => inspectedInstance.set(null)}
		on:keyup={() => inspectedInstance.set(null)}
	>
		<div class="flex flex-col">
			{#each { length: device.rows } as _, r}
				<div class="flex flex-row">
						{#each { length: device.columns } as _, c}
							<Key
								context={{ device: device.id, profile: profile.id, controller: "Keypad", position: (r * device.columns) + c }}
								bind:inslot={profile.keys[(r * device.columns) + c]}
								on:dragover={handleDragOver}
								on:drop={(event) => handleDrop(event, "Keypad", (r * device.columns) + c)}
								on:dragstart={(event) => handleDragStart(event, "Keypad", (r * device.columns) + c)}
								{handlePaste}
								size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
							/>
						{/each}
				</div>
			{/each}
		</div>

		<div class="flex flex-row">
			{#each { length: device.encoders } as _, i}
				<Key
					context={{ device: device.id, profile: profile.id, controller: "Encoder", position: i }}
					bind:inslot={profile.sliders[i]}
					on:dragover={handleDragOver}
					on:drop={(event) => handleDrop(event, "Encoder", i)}
					on:dragstart={(event) => handleDragStart(event, "Encoder", i)}
					{handlePaste}
					size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
				/>
			{/each}
		</div>
		<div class="flex flex-row">
			{#each { length: device.infobar } as _, i}
				<Key
					context={{ device: device.id, profile: profile.id, controller: "Infobar", position: i }}
					bind:inslot={profile.infobar[i]}
					on:dragover={handleDragOver}
					on:drop={(event) => handleDrop(event, "Infobar", i)}
					on:dragstart={(event) => handleDragStart(event, "Infobar", i)}
					{handlePaste}
					size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
					isInfobar
					deviceType={device.type}
				/>
			{/each}
		</div>
		<div class="flex flex-row">
			{#each { length: device.touchpoints } as _, i}
				<Key
					context={{ device: device.id, profile: profile.id, controller: "Keypad", position: (device.rows * device.columns) + i }}
					bind:inslot={profile.keys[(device.rows * device.columns) + i]}
					on:dragover={handleDragOver}
					on:drop={(event) => handleDrop(event, "Keypad", (device.rows * device.columns) + i)}
					on:dragstart={(event) => handleDragStart(event, "Keypad", (device.rows * device.columns) + i)}
					{handlePaste}
					size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
					isTouchPoint
				/>
			{/each}
		</div>
	</div>
{/key}
