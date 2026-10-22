<script lang="ts">
	import type { Action } from "$lib/Action";
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Context } from "$lib/Context";
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";
	import type { CopiedItem } from "$lib/propertyInspector";

	import Key from "./Key.svelte";

	import { t } from "$lib/i18n";
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
		else if (event.dataTransfer.types.includes("Files")) event.dataTransfer.dropEffect = "copy";
	}

	// Returns the name of a file dropped from outside the application, if any. WebKit
	// delivers dropped directories as a zip archive, so an application bundle arrives
	// as "Name.app.zip". Some webviews only expose bundles as a file URI instead.
	function droppedFileName(dataTransfer: DataTransfer): string | null {
		const file = dataTransfer.files?.[0];
		if (file?.name) return file.name;

		const uri = dataTransfer.getData("text/uri-list") || dataTransfer.getData("text/plain");
		if (!uri.startsWith("file://")) return null;

		const name = decodeURIComponent(uri.split(/\r?\n/)[0]).replace(/\/+$/, "").split("/").pop();
		return name || null;
	}

	async function handleDrop({ dataTransfer }: DragEvent, controller: string, position: number) {
		// The contents of a DataTransfer can only be read before the first await.
		const droppedFile = dataTransfer ? droppedFileName(dataTransfer) : null;
		const actionData = dataTransfer?.getData("action") ?? "";
		const controllerData = dataTransfer?.getData("controller") ?? "";
		const positionData = controllerData ? parseInt(dataTransfer?.getData("position") ?? "0") : 0;

		let context = { device: device.id, profile: profile.id, controller, position };
		let array = controller == "Encoder" ? profile.sliders : controller == "Infobar" ? profile.infobars : profile.keys;

		if (droppedFile) {
			// An application bundle or entry dragged in from the system file manager, from
			// which a Run Command action that launches the application can be created.
			let name: string | null = null;
			let command: string | null = null;
			if (droppedFile.endsWith(".app.zip")) {
				// WebKit delivers dropped directories as a zip archive, so an application
				// bundle arrives as "Name.app.zip".
				name = droppedFile.slice(0, -8);
				command = `open -a "${name}"`;
			} else if (droppedFile.endsWith(".app")) {
				name = droppedFile.slice(0, -4);
				command = `open -a "${name}"`;
			} else if (droppedFile.endsWith(".desktop")) {
				name = droppedFile.slice(0, -8);
				command = `gtk-launch "${name}"`;
			}
			if (command && !array[position]) {
				const categories: { [name: string]: { icon?: string; actions: Action[] } } = await invoke("get_categories");
				const action = Object.values(categories)
					.flatMap((category) => category.actions ?? [])
					.find((action) => action.uuid == "com.amansprojects.starterpack.runcommand");
				if (action) {
					const instance: ActionInstance = await invoke("create_instance", { context, action });
					if (instance) {
						instance.states[0].text_override = name;
						instance.states[0].show = true;
						const icon: string | null = name ? await invoke("get_application_icon", { name }) : null;
						if (icon) instance.states[0].image_override = icon;
						instance.settings = { down: command };
						array[position] = instance;
						profile = profile;
						await invoke("set_instance_settings", { context: instance.context, settings: instance.settings });
						await invoke("set_state", { context: instance.context, index: 0, state: instance.states[0] });
					}
				}
			}
		} else if (actionData) {
			let action = JSON.parse(actionData);
			if (array[position]) {
				return;
			}
			array[position] = await invoke("create_instance", { context, action });
			profile = profile;
		} else if (controllerData) {
			let oldController = controllerData;
			let oldArray = oldController == "Encoder" ? profile.sliders : oldController == "Infobar" ? profile.infobars : profile.keys;
			let response: ActionInstance = await invoke("move_instance", {
				source: { device: device.id, profile: profile.id, controller: oldController, position: positionData },
				destination: context,
				retain: false,
			});
			if (response) {
				array[position] = response;
				oldArray[positionData] = null;
				profile = profile;
			}
		}
	}

	async function handlePaste(item: CopiedItem, destination: Context) {
		let array = destination.controller == "Encoder" ? profile.sliders : destination.controller == "Infobar" ? profile.infobars : profile.keys;

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
	$: overflowsY = device.rows + Math.min(device.encoders, 1) + Math.min(device.touchpoints, 1) > 4;

	// Grid navigation: track focused cell and compute row lengths for arrow key movement.
	let focusedRow = 0;
	let focusedCol = 0;

	$: gridRowLengths = [
		...Array(device.rows).fill(device.columns),
		...(device.encoders > 0 ? [device.encoders] : []),
		...(device.touchpoints > 0 || device.infobars > 0 ? [device.touchpoints + device.infobars] : []),
	];
	$: encoderRowIndex = device.rows;
	$: touchpointRowIndex = device.rows + (device.encoders > 0 ? 1 : 0);
	$: keypadRowWidth = device.columns * 132;

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

{#key device}
	<span id="grid-description" class="sr-only">{$t("device_view.grid_description")}</span>
	<div
		class="flex flex-col justify-center grow px-16 py-6 overflow-auto"
		class:items-center={device.columns <= 9}
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
						<Key
							context={{ device: device.id, profile: profile.id, controller: "Keypad", position: r * device.columns + c }}
							bind:inslot={profile.keys[r * device.columns + c]}
							on:dragover={handleDragOver}
							on:drop={(event) => handleDrop(event, "Keypad", r * device.columns + c)}
							on:dragstart={(event) => handleDragStart(event, "Keypad", r * device.columns + c)}
							{handlePaste}
							size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
							label="{$t('device_view.key')} {String.fromCharCode(65 + r)}{c + 1}"
							tabindex={focusedRow === r && focusedCol === c ? 0 : -1}
						/>
					{/each}
				</div>
			{/each}
		</div>

		<div class="flex flex-row justify-between" role="row" style={`width: ${keypadRowWidth}px;`}>
			{#each { length: device.encoders } as _, i}
				<Key
					context={{ device: device.id, profile: profile.id, controller: "Encoder", position: i }}
					bind:inslot={profile.sliders[i]}
					on:dragover={handleDragOver}
					on:drop={(event) => handleDrop(event, "Encoder", i)}
					on:dragstart={(event) => handleDragStart(event, "Encoder", i)}
					{handlePaste}
					size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
					label="{$t('device_view.encoder')} {i + 1}"
					tabindex={focusedRow === encoderRowIndex && focusedCol === i ? 0 : -1}
				/>
			{/each}
		</div>

		<div class="flex flex-row items-center" role="row">
			{#each { length: device.touchpoints } as _, i}
				<!-- On the Stream Deck Neo, the infobar display sits physically between the two touchpoints. -->
				{#if device.infobars > 0 && i === 1}
					{#each { length: device.infobars } as _, j}
						<div class="px-3.5 py-[3.5px]">
							<Key
								context={{ device: device.id, profile: profile.id, controller: "Infobar", position: j }}
								bind:inslot={profile.infobars[j]}
								on:dragover={handleDragOver}
								on:drop={(event) => handleDrop(event, "Infobar", j)}
								on:dragstart={(event) => handleDragStart(event, "Infobar", j)}
								{handlePaste}
								size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
								width={248}
								height={58}
							/>
						</div>
					{/each}
				{/if}
				<Key
					context={{ device: device.id, profile: profile.id, controller: "Keypad", position: device.rows * device.columns + i }}
					bind:inslot={profile.keys[device.rows * device.columns + i]}
					on:dragover={handleDragOver}
					on:drop={(event) => handleDrop(event, "Keypad", device.rows * device.columns + i)}
					on:dragstart={(event) => handleDragStart(event, "Keypad", device.rows * device.columns + i)}
					{handlePaste}
					size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
					isTouchPoint
					label="{$t('device_view.touchpoint')} {i + 1}"
					tabindex={focusedRow === touchpointRowIndex && focusedCol === i ? 0 : -1}
				/>
			{/each}
		</div>
	</div>
{/key}

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
