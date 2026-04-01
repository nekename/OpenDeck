<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Context } from "$lib/Context";
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";
	import type { CopiedItem } from "$lib/propertyInspector";

	import Key from "./Key.svelte";

	import { inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";
	import { settings } from "$lib/settings";

	import { invoke } from "@tauri-apps/api/core";

	export let device: DeviceInfo;
	export let profile: Profile;

	export let selectedDevice: string;
	let dragPreviewEl: HTMLCanvasElement | null = null;

	function getDragSourceCanvas(event: DragEvent): HTMLCanvasElement | null {
		if (event.target instanceof HTMLCanvasElement) return event.target;
		if (event.currentTarget instanceof HTMLCanvasElement) return event.currentTarget;
		for (const node of event.composedPath()) {
			if (node instanceof HTMLCanvasElement) return node;
		}
		return null;
	}

	function cleanupDragPreview() {
		if (dragPreviewEl?.parentElement) dragPreviewEl.parentElement.removeChild(dragPreviewEl);
		dragPreviewEl = null;
	}

	function createRotatedDragPreview(source: HTMLCanvasElement, rotation: number): HTMLCanvasElement | null {
		const normalizedRotation = ((rotation % 360) + 360) % 360;
		if (normalizedRotation === 0) return null;

		const swapSides = normalizedRotation === 90 || normalizedRotation === 270;
		const preview = document.createElement("canvas");
		preview.width = swapSides ? source.height : source.width;
		preview.height = swapSides ? source.width : source.height;

		const context = preview.getContext("2d");
		if (!context) return null;

		context.translate(preview.width / 2, preview.height / 2);
		context.rotate((-normalizedRotation * Math.PI) / 180);
		context.drawImage(source, -source.width / 2, -source.height / 2);

		// Keep the preview in the DOM so WebView drag image rendering stays reliable.
		preview.style.position = "fixed";
		preview.style.left = "-9999px";
		preview.style.top = "0";
		preview.style.pointerEvents = "none";
		document.body.appendChild(preview);

		return preview;
	}

	function handleDragStart(event: DragEvent, controller: string, position: number) {
		const { dataTransfer } = event;
		if (!dataTransfer) return;
		cleanupDragPreview();
		dataTransfer.effectAllowed = "move";
		dataTransfer.setData("controller", controller);
		dataTransfer.setData("position", position.toString());
		dataTransfer.setData("text/plain", `${controller}:${position}`);

		const source = getDragSourceCanvas(event);
		if (!source) return;

		const preview = createRotatedDragPreview(source, visualRotation);
		if (preview) {
			dragPreviewEl = preview;
			dataTransfer.setDragImage(preview, preview.width / 2, preview.height / 2);
			return;
		}

		// Fallback to native ghost image when no rotation is needed or preview creation failed.
		dataTransfer.setDragImage(source, source.width / 2, source.height / 2);
	}

	function handleDragEnd() {
		cleanupDragPreview();
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		if (!event.dataTransfer) return;
		if (event.dataTransfer.types.includes("action")) event.dataTransfer.dropEffect = "copy";
		else if (event.dataTransfer.types.includes("controller")) event.dataTransfer.dropEffect = "move";
	}

	async function handleDrop({ dataTransfer }: DragEvent, controller: string, position: number) {
		let context = { device: device.id, profile: profile.id, controller, position };
		let array = controller == "Encoder" ? profile.sliders : profile.keys;
		if (dataTransfer?.getData("action")) {
			let action = JSON.parse(dataTransfer?.getData("action"));
			if (array[position]) {
				return;
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

	$: maxColumns = Math.max(device.columns, device.encoders, device.touchpoints);
	$: totalRows = device.rows + Math.min(device.encoders, 1) + Math.min(device.touchpoints, 1);
	$: visualRotation = $settings?.device_rotation ?? 0;
	$: rotatedPortrait = visualRotation == 90 || visualRotation == 270;
	$: visibleColumns = rotatedPortrait ? totalRows : maxColumns;
	$: visibleRows = rotatedPortrait ? maxColumns : totalRows;
	$: overflowsX = visibleColumns > 8;
	$: overflowsY = visibleRows > 4;
	$: deviceWidth = maxColumns * 132;
	$: deviceHeight = totalRows * 132;
	$: rotatedWidth = rotatedPortrait ? deviceHeight : deviceWidth;
	$: rotatedHeight = rotatedPortrait ? deviceWidth : deviceHeight;

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
		class:items-center={visibleColumns <= 8}
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
		on:dragend={handleDragEnd}
		on:keydown|capture={handleGridKeydown}
		on:focusin={handleGridFocusin}
	>
		<div class="relative" style={`width: ${rotatedWidth}px; height: ${rotatedHeight}px;`}>
			<div class="absolute left-1/2 top-1/2" style={`transform: translate(-50%, -50%) rotate(${-visualRotation}deg);`}>
				<div class="flex flex-col" role="rowgroup">
					{#each { length: device.rows } as _, r}
						<div class="flex flex-row" role="row">
							{#each { length: device.columns } as _, c}
								<Key
									context={{ device: device.id, profile: profile.id, controller: "Keypad", position: (r * device.columns) + c }}
									bind:inslot={profile.keys[(r * device.columns) + c]}
									on:dragover={handleDragOver}
									on:drop={(event) => handleDrop(event, "Keypad", (r * device.columns) + c)}
									on:dragstart={(event) => handleDragStart(event, "Keypad", (r * device.columns) + c)}
									{handlePaste}
									size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
									label="Key {String.fromCharCode(65 + r)}{c + 1}"
									tabindex={focusedRow === r && focusedCol === c ? 0 : -1}
								/>
							{/each}
						</div>
					{/each}
				</div>

				<div class="flex flex-row" role="row">
					{#each { length: device.encoders } as _, i}
						<Key
							context={{ device: device.id, profile: profile.id, controller: "Encoder", position: i }}
							bind:inslot={profile.sliders[i]}
							on:dragover={handleDragOver}
							on:drop={(event) => handleDrop(event, "Encoder", i)}
							on:dragstart={(event) => handleDragStart(event, "Encoder", i)}
							{handlePaste}
							size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
							label="Encoder {i + 1}"
							tabindex={focusedRow === encoderRowIndex && focusedCol === i ? 0 : -1}
						/>
					{/each}
				</div>

				<div class="flex flex-row" role="row">
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
							label="Touch point {i + 1}"
							tabindex={focusedRow === touchpointRowIndex && focusedCol === i ? 0 : -1}
						/>
					{/each}
				</div>
			</div>
		</div>
	</div>
{/key}
