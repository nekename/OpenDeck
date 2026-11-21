<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Context } from "$lib/Context";
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";
	import type { CopiedItem } from "$lib/propertyInspector";

	import Key from "./Key.svelte";

	import Plus from "phosphor-svelte/lib/Plus";
	import Trash from "phosphor-svelte/lib/Trash";

	import { t } from "$lib/i18n";
	import { inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { tick } from "svelte";

	export let device: DeviceInfo;
	export let profile: Profile;

	export let selectedDevice: string;

	function handleDragStart({ dataTransfer }: DragEvent, controller: string, position: number) {
		if (!dataTransfer) return;
		dataTransfer.effectAllowed = "move";
		dataTransfer.setData("controller", controller);
		dataTransfer.setData("position", position.toString());
		dataTransfer.setData("page", currentPage.toString());
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		if (!event.dataTransfer) return;
		if (event.dataTransfer.types.includes("action")) event.dataTransfer.dropEffect = "copy";
		else if (event.dataTransfer.types.includes("controller")) event.dataTransfer.dropEffect = "move";
	}

	$: currentPage = profile.current_page ?? 0;
	$: pageCount = profile.pages?.length || 1;
	$: currentSlots = profile.pages?.[currentPage] ?? profile;

	// Pass page, profile, and device in from the call site. Svelte only tracks
	// identifiers written in the template, not values this function closes over.
	// A context frozen at first render keeps page 0, and the backend then drops
	// every image for a later page, so the deck stays blank after the clear.
	function slotContext(controller: string, position: number, page: number, profileId: string, deviceId: string): Context {
		return { device: deviceId, profile: profileId, page, controller, position };
	}

	function writeSlot(controller: string, position: number, value: ActionInstance | null) {
		const page = profile.pages?.[currentPage];
		if (controller == "Encoder") {
			if (page) page.sliders[position] = value;
			profile.sliders[position] = value;
		} else if (controller == "Infobar") {
			if (page) page.infobars[position] = value;
			profile.infobars[position] = value;
		} else {
			if (page) page.keys[position] = value;
			profile.keys[position] = value;
		}
		profile = profile;
	}

	// switch_page carries the profile from before plugins send setImage. Apply it, let keys
	// bind, then ask the backend to willAppear. A second apply would put the stale snapshot back.
	let showGen = 0;
	async function showProfile(next: Profile) {
		const gen = ++showGen;
		const page = next.current_page ?? 0;
		profile = next;
		await tick();
		if (gen !== showGen) return;
		await invoke("refresh_current_page", { device: device.id, page });
	}

	listen("switch_page", ({ payload }: { payload: { device: string; profile: Profile } }) => {
		if (payload.device == device.id) void showProfile(payload.profile);
	});

	async function runPageCommand(command: Promise<Profile>) {
		const genAtStart = showGen;
		const next = await command;
		if (showGen === genAtStart) await showProfile(next);
		$inspectedInstance = null;
		pageMenu = null;
	}

	async function selectPage(index: number) {
		if (index == currentPage) return;
		await runPageCommand(invoke("set_selected_page", { device: device.id, page: index }));
	}

	async function addPage() {
		await runPageCommand(invoke("add_profile_page", { device: device.id }));
	}

	async function deletePage(index: number) {
		if (pageCount <= 1) return;
		await runPageCommand(invoke("delete_profile_page", { device: device.id, page: index }));
	}

	let pageMenu: { index: number; x: number; y: number } | null = null;
	let pageMenuEl: HTMLDivElement;
	async function openPageMenu(event: MouseEvent, index: number) {
		event.preventDefault();
		event.stopPropagation();
		if (pageCount <= 1) return;
		pageMenu = { index, x: event.clientX, y: event.clientY };
		await tick();
		pageMenuEl?.querySelector("button")?.focus();
	}

	async function handleDrop({ dataTransfer }: DragEvent, controller: string, position: number) {
		let context = slotContext(controller, position, currentPage, profile.id, device.id);
		let array = controller == "Encoder" ? currentSlots.sliders : controller == "Infobar" ? currentSlots.infobars : currentSlots.keys;
		if (dataTransfer?.getData("action")) {
			let action = JSON.parse(dataTransfer?.getData("action"));
			if (array[position]) {
				return;
			}
			writeSlot(controller, position, await invoke("create_instance", { context, action }));
		} else if (dataTransfer?.getData("controller")) {
			let oldController = dataTransfer?.getData("controller");
			let oldPosition = parseInt(dataTransfer?.getData("position"));
			let oldPage = parseInt(dataTransfer?.getData("page") || currentPage.toString());
			let response: ActionInstance = await invoke("move_instance", {
				source: { device: device.id, profile: profile.id, page: oldPage, controller: oldController, position: oldPosition },
				destination: context,
				retain: false,
			});
			if (response) {
				writeSlot(controller, position, response);
				if (oldPage == currentPage) writeSlot(oldController, oldPosition, null);
			}
		}
	}

	async function handlePaste(item: CopiedItem, destination: Context) {
		let array = destination.controller == "Encoder" ? currentSlots.sliders : destination.controller == "Infobar" ? currentSlots.infobars : currentSlots.keys;

		if (item.type == "action") {
			if (array[destination.position]) return;
			writeSlot(destination.controller, destination.position, await invoke("create_instance", { context: destination, action: item.action }));
			return;
		}

		let response: ActionInstance = await invoke("move_instance", { source: item.source, destination, retain: true });
		if (response) writeSlot(destination.controller, destination.position, response);
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
						context={slotContext("Keypad", r * device.columns + c, currentPage, profile.id, device.id)}
						inslot={currentSlots.keys[r * device.columns + c]}
						on:cleared={() => writeSlot("Keypad", r * device.columns + c, null)}
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
				context={slotContext("Encoder", i, currentPage, profile.id, device.id)}
				inslot={currentSlots.sliders[i]}
				on:cleared={() => writeSlot("Encoder", i, null)}
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
							context={slotContext("Infobar", j, currentPage, profile.id, device.id)}
							inslot={currentSlots.infobars[j]}
							on:cleared={() => writeSlot("Infobar", j, null)}
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
				context={slotContext("Keypad", device.rows * device.columns + i, currentPage, profile.id, device.id)}
				inslot={currentSlots.keys[device.rows * device.columns + i]}
				on:cleared={() => writeSlot("Keypad", device.rows * device.columns + i, null)}
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

	<div class="flex flex-row justify-center items-center flex-wrap gap-2 mt-5" role="tablist" aria-label={$t("device_view.pages")}>
		{#each Array(pageCount) as _, index}
			<button
				type="button"
				role="tab"
				aria-selected={index == currentPage}
				aria-label={$t("device_view.page", { n: index + 1 })}
				class="min-w-8 h-8 px-3 rounded-full text-sm font-semibold border transition-colors"
				class:bg-neutral-200={index == currentPage}
				class:text-neutral-900={index == currentPage}
				class:border-neutral-200={index == currentPage}
				class:bg-neutral-800={index != currentPage}
				class:text-neutral-300={index != currentPage}
				class:border-neutral-600={index != currentPage}
				class:hover:bg-neutral-700={index != currentPage}
				on:click|stopPropagation={() => selectPage(index)}
				on:contextmenu={(event) => openPageMenu(event, index)}
			>
				{index + 1}
			</button>
		{/each}
		<button
			type="button"
			class="w-8 h-8 flex items-center justify-center rounded-full text-neutral-300 bg-neutral-800 border border-neutral-600 hover:bg-neutral-700 transition-colors"
			on:click|stopPropagation={addPage}
			aria-label={$t("device_view.page.add")}
			title={$t("device_view.page.add")}
		>
			<Plus size={16} />
		</button>
	</div>
</div>

{#if pageMenu}
	<div
		bind:this={pageMenuEl}
		class="fixed w-40 font-semibold text-sm text-neutral-300 bg-neutral-700 border border-neutral-600 rounded-lg z-20"
		style={`left: ${pageMenu.x}px; top: ${pageMenu.y}px;`}
	>
		<button
			class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-lg cursor-pointer"
			on:click|stopPropagation={() => {
				if (pageMenu) deletePage(pageMenu.index);
			}}
		>
			<Trash size="18" class="text-red-400" />
			<span class="ml-2">{$t("device_view.page.delete")}</span>
		</button>
	</div>
{/if}

<svelte:window on:click={() => (pageMenu = null)} />

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
