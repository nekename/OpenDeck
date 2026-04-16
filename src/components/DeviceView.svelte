<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Context } from "$lib/Context";
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";
	import type { CopiedItem } from "$lib/propertyInspector";

	import Key from "./Key.svelte";

	import { inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";
	import { dragAction, hoveredSlot } from "$lib/dragState";

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

	function handleDragOver(event: DragEvent, controller?: string, position?: number) {
		event.preventDefault();
		if (!event.dataTransfer) return;
		if (event.dataTransfer.types.includes("action")) event.dataTransfer.dropEffect = "copy";
		else if (event.dataTransfer.types.includes("controller")) event.dataTransfer.dropEffect = "move";
		if (controller != null && position != null) hoveredSlot.set({ controller, position });
	}

	async function handleDrop({ dataTransfer }: DragEvent, controller: string, position: number) {
		hoveredSlot.set(null);
		let context = { device: device.id, profile: profile.id, controller, position };
		let array = controller == "Encoder" ? profile.sliders : profile.keys;
		if (dataTransfer?.getData("action")) {
			let action = JSON.parse(dataTransfer?.getData("action"));
			// Allow dropping on occupied slots by removing the existing action first
			if (array[position]) {
				await invoke("remove_instance", { context: array[position].context });
				array[position] = null;
			}
			array[position] = await invoke("create_instance", { context, action });
			profile = profile;
		} else if (dataTransfer?.getData("controller")) {
			let oldArray = dataTransfer?.getData("controller") == "Encoder" ? profile.sliders : profile.keys;
			let oldPosition = parseInt(dataTransfer?.getData("position"));
			let response: ActionInstance = await invoke("move_instance", {
				source: { device: device.id, profile: profile.id, controller: dataTransfer?.getData("controller"), position: oldPosition },
				destination: context,
				retain: false,
			});
			if (response) {
				array[position] = response;
				oldArray[oldPosition] = null;
				profile = profile;
			}
		}
	}

	async function handlePaste(item: CopiedItem, destination: Context) {
		let array = destination.controller == "Encoder" ? profile.sliders : profile.keys;

		if (item.type == "action") {
			if (array[destination.position]) return;
			array[destination.position] = await invoke("create_instance", { context: destination, action: item.action });
			profile = profile;
			return;
		}

		let response: ActionInstance = await invoke("move_instance", { source: item.source, destination, retain: true });
		if (response) {
			array[destination.position] = response;
			profile = profile;
		}
	}

	$: overflowsX = Math.max(device.columns, device.encoders, device.touchpoints) > 8;
	$: overflowsY = (device.rows + Math.min(device.encoders, 1) + Math.min(device.touchpoints, 1)) > 4;

	// Grid navigation: track focused cell and compute row lengths for arrow key movement.
	let focusedRow = 0;
	let focusedCol = 0;

	$: gridRowLengths = [
		...Array(device.rows).fill(device.columns),
		...(device.encoders > 0 ? [device.encoders] : []),
		...(device.touchpoints > 0 ? [device.touchpoints] : []),
	];
	$: encoderRowIndex = device.rows;
	$: touchpointRowIndex = device.rows + (device.encoders > 0 ? 1 : 0);

	function flatIndexFromRowCol(row: number, col: number): number {
		let index = 0;
		for (let r = 0; r < row; r++) index += gridRowLengths[r];
		return index + col;
	}

	function rowColFromFlatIndex(flatIndex: number): [number, number] {
		let remaining = flatIndex;
		for (let r = 0; r < gridRowLengths.length; r++) {
			if (remaining < gridRowLengths[r]) return [r, remaining];
			remaining -= gridRowLengths[r];
		}
		return [0, 0];
	}

	function handleGridKeydown(event: KeyboardEvent) {
		const target = event.target as HTMLElement;
		if (target.getAttribute("role") !== "gridcell") return;
		if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;

		event.preventDefault();
		event.stopPropagation();

		let newRow = focusedRow;
		let newCol = focusedCol;

		switch (event.key) {
			case "ArrowRight":
				newCol = Math.min(focusedCol + 1, gridRowLengths[focusedRow] - 1);
				break;
			case "ArrowLeft":
				newCol = Math.max(focusedCol - 1, 0);
				break;
			case "ArrowDown":
				newRow = Math.min(focusedRow + 1, gridRowLengths.length - 1);
				newCol = Math.min(focusedCol, gridRowLengths[newRow] - 1);
				break;
			case "ArrowUp":
				newRow = Math.max(focusedRow - 1, 0);
				newCol = Math.min(focusedCol, gridRowLengths[newRow] - 1);
				break;
			case "Home":
				newCol = 0;
				break;
			case "End":
				newCol = gridRowLengths[focusedRow] - 1;
				break;
		}

		if (newRow === focusedRow && newCol === focusedCol) return;

		focusedRow = newRow;
		focusedCol = newCol;

		const grid = event.currentTarget as HTMLElement;
		const cells = grid.querySelectorAll("[role='gridcell']");
		(cells[flatIndexFromRowCol(newRow, newCol)] as HTMLElement)?.focus();
	}

	function handleGridFocusin(event: FocusEvent) {
		const grid = event.currentTarget as HTMLElement;
		const cells = Array.from(grid.querySelectorAll("[role='gridcell']"));
		const index = cells.indexOf(event.target as Element);
		if (index === -1) return;
		[focusedRow, focusedCol] = rowColFromFlatIndex(index);
	}
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
	<span id="grid-description" class="sr-only">Use arrow keys to navigate between keys. Moving to a key will display its property inspector.</span>
	<div
		class="flex flex-col justify-center grow px-16 py-6 overflow-auto"
		class:items-center={device.columns <= 8}
		class:hidden={$inspectedParentAction || selectedDevice != device.id}
		class:device-fade-x={overflowsX && !overflowsY}
		class:device-fade-y={overflowsY && !overflowsX}
		class:device-fade-xy={overflowsX && overflowsY}
		role="grid"
		aria-label={device.name}
		aria-describedby="grid-description"
		tabindex="-1"
		on:click={() => inspectedInstance.set(null)}
		on:keyup={() => inspectedInstance.set(null)}
		on:keydown|capture={handleGridKeydown}
		on:focusin={handleGridFocusin}
	>
		<div class="flex flex-col" role="rowgroup">
			{#each { length: device.rows } as _, r}
				<div class="flex flex-row" role="row">
					{#each { length: device.columns } as _, c}
						{@const pos = (r * device.columns) + c}
						{@const isCompat = $dragAction ? $dragAction.controllers.includes("Keypad") : false}
						{@const isHovered = $hoveredSlot?.controller === "Keypad" && $hoveredSlot?.position === pos}
						{@const isEmpty = !profile.keys[pos]}
						<Key
							context={{ device: device.id, profile: profile.id, controller: "Keypad", position: pos }}
							bind:inslot={profile.keys[pos]}
							on:dragover={(event) => handleDragOver(event, "Keypad", pos)}
							on:dragleave={() => hoveredSlot.set(null)}
							on:drop={(event) => handleDrop(event, "Keypad", pos)}
							on:dragstart={(event) => handleDragStart(event, "Keypad", pos)}
							{handlePaste}
							size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
							label="Key {String.fromCharCode(65 + r)}{c + 1}"
							tabindex={focusedRow === r && focusedCol === c ? 0 : -1}
							dragHighlight={$dragAction ? (isCompat ? (isHovered ? "hovered" : (isEmpty ? "empty" : "occupied")) : "incompatible") : null}
						/>
					{/each}
				</div>
			{/each}
		</div>

		{#if device.encoders > 0}
			<div class="flex flex-row items-start justify-center mt-2 gap-0" role="row" style="width: {device.columns <= 8 ? (device.columns * 132) : (device.columns * 144)}px; margin: 0 auto;">
				{#each { length: device.encoders } as _, i}
					{@const isCompat = $dragAction ? $dragAction.controllers.includes("Encoder") : false}
					{@const isHovered = $hoveredSlot?.controller === "Encoder" && $hoveredSlot?.position === i}
					{@const isEmpty = !profile.sliders[i]}
					{@const encHighlight = $dragAction ? (isCompat ? (isHovered ? "hovered" : (isEmpty ? "empty" : "occupied")) : "incompatible") : null}
					<div
						class="flex flex-col items-center transition-all duration-150"
						class:opacity-30={$dragAction && !isCompat}
						class:brightness-125={isHovered}
						style="flex: 1;"
						on:dragover|preventDefault={(event) => handleDragOver(event, "Encoder", i)}
						on:dragleave={() => hoveredSlot.set(null)}
						on:drop={(event) => handleDrop(event, "Encoder", i)}
					>
						<Key
							context={{ device: device.id, profile: profile.id, controller: "Encoder", position: i }}
							bind:inslot={profile.sliders[i]}
							on:dragstart={(event) => handleDragStart(event, "Encoder", i)}
							{handlePaste}
							encoderStrip
							encoderPosition={i}
							encoderCount={device.encoders}
							label="Encoder {i + 1}"
							tabindex={focusedRow === encoderRowIndex && focusedCol === i ? 0 : -1}
							dragHighlight={encHighlight}
						/>
					</div>
				{/each}
			</div>
		{/if}

		<div class="flex flex-row" role="row">
			{#each { length: device.touchpoints } as _, i}
				{@const tpos = (device.rows * device.columns) + i}
				{@const isCompat = $dragAction ? $dragAction.controllers.includes("Keypad") : false}
				{@const isHovered = $hoveredSlot?.controller === "Keypad" && $hoveredSlot?.position === tpos}
				{@const isEmpty = !profile.keys[tpos]}
				<Key
					context={{ device: device.id, profile: profile.id, controller: "Keypad", position: tpos }}
					bind:inslot={profile.keys[tpos]}
					on:dragover={(event) => handleDragOver(event, "Keypad", tpos)}
					on:dragleave={() => hoveredSlot.set(null)}
					on:drop={(event) => handleDrop(event, "Keypad", tpos)}
					on:dragstart={(event) => handleDragStart(event, "Keypad", tpos)}
					dragHighlight={$dragAction ? (isCompat ? (isHovered ? "hovered" : (isEmpty ? "empty" : "occupied")) : "incompatible") : null}
					{handlePaste}
					size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
					isTouchPoint
					label="Touch point {i + 1}"
					tabindex={focusedRow === touchpointRowIndex && focusedCol === i ? 0 : -1}
				/>
			{/each}
		</div>
	</div>
{/key}
