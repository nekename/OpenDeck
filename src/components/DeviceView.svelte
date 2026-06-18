<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Context } from "$lib/Context";
	import type { DeviceDescriptor, DeviceLayout, DeviceLayoutControl, DeviceLayoutSurface } from "$lib/DeviceInfo";
	import { SUPPORTED_DEVICE_LAYOUT_VERSION } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";
	import type { CopiedItem } from "$lib/propertyInspector";

	import Key from "./Key.svelte";

	import { inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";

	export let device: DeviceDescriptor;
	export let profile: Profile;

	export let selectedDevice: string;

	const DEVICE_LAYOUT_VIEWPORT_PADDING_X = 128;
	const DEVICE_LAYOUT_VIEWPORT_PADDING_Y = 48;
	const DEVICE_LAYOUT_ROW_TOLERANCE = 24;

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

	$: overflowsX = Math.max(device.columns, device.encoders, device.touchpoints) > 8;
	$: overflowsY = (device.rows + Math.min(device.encoders, 1) + Math.min(device.touchpoints, 1)) > 4;
	$: explicitLayout = device.layoutVersion == SUPPORTED_DEVICE_LAYOUT_VERSION && isDeviceLayout(device.layout) ? device.layout : undefined;
	$: explicitControls = (explicitLayout?.controls ?? []).filter(isSupportedControl);
	$: explicitNavigationRows = buildNavigationRows(explicitControls);
	$: navigationOrderedControls = explicitNavigationRows.flat();
	$: surfaceLookup = new Map((explicitLayout?.surfaces ?? []).map((surface) => [surface.id, surface]));
	let layoutViewportWidth = 0;
	let layoutViewportHeight = 0;
	$: explicitScale = explicitLayout ? calculateLayoutScale(explicitLayout, layoutViewportWidth, layoutViewportHeight) : 1;

	// Grid navigation: track focused cell and compute row lengths for arrow key movement.
	let focusedRow = 0;
	let focusedCol = 0;

	$: gridRowLengths = explicitLayout ? explicitNavigationRows.map((row) => row.length) : [
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

	function isDeviceLayout(value: unknown): value is DeviceLayout {
		if (!value || typeof value !== "object") return false;
		const candidate = value as DeviceLayout;
		return Number.isFinite(candidate.canvas?.width)
			&& Number.isFinite(candidate.canvas?.height)
			&& candidate.canvas.width > 0
			&& candidate.canvas.height > 0
			&& Array.isArray(candidate.controls);
	}

	function isSupportedControl(control: DeviceLayoutControl): boolean {
		return (control.controller == "Keypad" || control.controller == "Encoder") && controlWidth(control) > 0 && controlHeight(control) > 0;
	}

	function calculateLayoutScale(layout: DeviceLayout, viewportWidth: number, viewportHeight: number): number {
		const availableWidth = Math.max(0, viewportWidth - DEVICE_LAYOUT_VIEWPORT_PADDING_X);
		const availableHeight = Math.max(0, viewportHeight - DEVICE_LAYOUT_VIEWPORT_PADDING_Y);
		if (availableWidth == 0 || availableHeight == 0) return 1;
		return Math.min(1, availableWidth / layout.canvas.width, availableHeight / layout.canvas.height);
	}

	function controlX(control: DeviceLayoutControl): number {
		return control.shape == "circle" ? control.cx - control.r : control.x;
	}

	function controlY(control: DeviceLayoutControl): number {
		return control.shape == "circle" ? control.cy - control.r : control.y;
	}

	function controlWidth(control: DeviceLayoutControl): number {
		return control.shape == "circle" ? control.r * 2 : control.width;
	}

	function controlHeight(control: DeviceLayoutControl): number {
		return control.shape == "circle" ? control.r * 2 : control.height;
	}

	function controlCenterX(control: DeviceLayoutControl): number {
		return controlX(control) + (controlWidth(control) / 2);
	}

	function controlCenterY(control: DeviceLayoutControl): number {
		return controlY(control) + (controlHeight(control) / 2);
	}

	function buildNavigationRows(controls: DeviceLayoutControl[]): DeviceLayoutControl[][] {
		const sorted = [...controls].sort((a, b) => controlCenterY(a) - controlCenterY(b) || controlCenterX(a) - controlCenterX(b));
		const rows: DeviceLayoutControl[][] = [];
		for (const control of sorted) {
			const row = rows.find((candidate) => Math.abs(controlCenterY(candidate[0]) - controlCenterY(control)) <= DEVICE_LAYOUT_ROW_TOLERANCE);
			if (row) row.push(control);
			else rows.push([control]);
		}
		for (const row of rows) row.sort((a, b) => controlCenterX(a) - controlCenterX(b));
		return rows;
	}

	function controlStyle(control: DeviceLayoutControl): string {
		return `left: ${controlX(control)}px; top: ${controlY(control)}px; width: ${controlWidth(control)}px; height: ${controlHeight(control)}px;`;
	}

	function surfaceStyle(surface: DeviceLayoutSurface): string {
		return `left: ${surface.x}px; top: ${surface.y}px; width: ${surface.width}px; height: ${surface.height}px;`;
	}

	function controlSurface(control: DeviceLayoutControl): DeviceLayoutSurface | undefined {
		return control.surface ? surfaceLookup.get(control.surface) : undefined;
	}

	function isTouchPointControl(control: DeviceLayoutControl): boolean {
		return control.controller == "Keypad" && controlSurface(control)?.kind == "touchpanel";
	}

	function showButtonIndicator(control: DeviceLayoutControl): boolean {
		return control.controller == "Keypad" && control.shape == "circle";
	}

	function controlLabel(control: DeviceLayoutControl): string {
		if (control.controller == "Encoder") return `Encoder ${control.position + 1}`;
		if (isTouchPointControl(control)) return `Touch point ${control.position + 1}`;
		return `Key ${control.position + 1}`;
	}

	function selectedTouchPanelSurfaces(): DeviceLayoutSurface[] {
		if (!explicitLayout) return [];
		const selectedSurfaces = new Set<string>();
		for (const control of explicitControls) {
			if (!isTouchPointControl(control) || !control.surface) continue;
			const context = `${device.id}.${profile.id}.Keypad.${control.position}.0`;
			if ($inspectedInstance == context || JSON.stringify($inspectedInstance) == JSON.stringify({ device: device.id, profile: profile.id, controller: "Keypad", position: control.position })) {
				selectedSurfaces.add(control.surface);
			}
		}
		return (explicitLayout.surfaces ?? []).filter((surface) => selectedSurfaces.has(surface.id));
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
	.device-layout-surface {
		background-color: rgb(38 38 38 / 0.35);
		border: 3px solid rgb(64 64 64);
		border-radius: 1.5rem;
		pointer-events: none;
		position: absolute;
	}
	.device-layout-surface-touchpanel::after {
		border-top: 4px solid rgb(64 64 64);
		content: "";
		left: 25%;
		position: absolute;
		top: 50%;
		width: 50%;
	}
	.device-layout-surface-selected {
		background-color: rgb(59 130 246 / 0.15);
		outline: 2px solid rgb(59 130 246);
		outline-offset: 2px;
	}
</style>

{#key device}
	<span id="grid-description" class="sr-only">Use arrow keys to navigate between keys. Moving to a key will display its property inspector.</span>
	<div
		class="flex flex-col justify-center grow px-16 py-6 overflow-auto"
		class:items-center={explicitLayout || device.columns <= 8}
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
		bind:clientWidth={layoutViewportWidth}
		bind:clientHeight={layoutViewportHeight}
	>
		{#if explicitLayout}
			<div
				class="relative shrink-0"
				style={`width: ${explicitLayout.canvas.width}px; height: ${explicitLayout.canvas.height}px; transform: scale(${explicitScale}); transform-origin: center;`}
				role="rowgroup"
			>
				{#each (explicitLayout.surfaces ?? []).filter((surface) => surface.kind == "touchpanel") as surface}
					<div
						class="device-layout-surface device-layout-surface-touchpanel"
						style={surfaceStyle(surface)}
						aria-hidden="true"
					/>
				{/each}
				{#each selectedTouchPanelSurfaces() as surface}
					<div
						class="device-layout-surface device-layout-surface-selected"
						style={surfaceStyle(surface)}
						aria-hidden="true"
					/>
				{/each}

				{#each navigationOrderedControls as control, index}
					<div class="absolute" style={controlStyle(control)}>
						{#if control.controller == "Encoder"}
							<Key
								context={{ device: device.id, profile: profile.id, controller: "Encoder", position: control.position }}
								bind:inslot={profile.sliders[control.position]}
								on:dragover={handleDragOver}
								on:drop={(event) => handleDrop(event, "Encoder", control.position)}
								on:dragstart={(event) => handleDragStart(event, "Encoder", control.position)}
								{handlePaste}
								width={controlWidth(control)}
								height={controlHeight(control)}
								directSize
								label={controlLabel(control)}
								tabindex={focusedRow === 0 && focusedCol === index ? 0 : -1}
							/>
						{:else}
							<Key
								context={{ device: device.id, profile: profile.id, controller: "Keypad", position: control.position }}
								bind:inslot={profile.keys[control.position]}
								on:dragover={handleDragOver}
								on:drop={(event) => handleDrop(event, "Keypad", control.position)}
								on:dragstart={(event) => handleDragStart(event, "Keypad", control.position)}
								{handlePaste}
								width={controlWidth(control)}
								height={controlHeight(control)}
								directSize
								isTouchPoint={isTouchPointControl(control)}
								isTouchPanelZone={isTouchPointControl(control)}
								showButtonIndicator={showButtonIndicator(control)}
								label={controlLabel(control)}
								tabindex={focusedRow === 0 && focusedCol === index ? 0 : -1}
							/>
						{/if}
					</div>
				{/each}
			</div>
		{:else}
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
		{/if}
	</div>
{/key}
