<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Context } from "$lib/Context";
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import Key from "./Key.svelte";

	import { copiedContext, inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";
	import { createEventDispatcher } from "svelte";

	const dispatch = createEventDispatcher();

	export let device: DeviceInfo;
	export let profile: Profile;

	export let selectedDevice: string;

	function handleDragStart({ dataTransfer }: DragEvent, controller: string, position: number) {
		dataTransfer?.setData("controller", controller);
		dataTransfer?.setData("position", position.toString());
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		return true;
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

	async function handlePaste(source: Context, destination: Context) {
		let response: ActionInstance = await invoke("move_instance", { source, destination, retain: true });
		if (response) {
			(destination.controller == "Encoder" ? profile.sliders : profile.keys)[destination.position] = response;
			profile = profile;
		}
	}

	// Accessibility: Handle keyboard-based action assignment
	let selectedAction: any = null;
	let focusedKeyIndex: number = -1;
	let allKeyContexts: Context[] = [];
	let announcementEl: HTMLElement;

	// Calculate all key contexts for keyboard navigation
	$: {
		allKeyContexts = [];
		// Add keypad keys
		for (let r = 0; r < device.rows; r++) {
			for (let c = 0; c < device.columns; c++) {
				allKeyContexts.push({
					device: device.id,
					profile: profile.id,
					controller: "Keypad",
					position: (r * device.columns) + c,
				});
			}
		}
		// Add encoder keys
		for (let i = 0; i < device.encoders; i++) {
			allKeyContexts.push({
				device: device.id,
				profile: profile.id,
				controller: "Encoder",
				position: i,
			});
		}
	}

	function announceToScreenReader(message: string) {
		if (announcementEl) {
			announcementEl.textContent = message;
		}
	}

	function handleDeviceKeyDown(event: KeyboardEvent) {
		if (allKeyContexts.length === 0) return;

		// Initialize focus to first key if not set
		if (focusedKeyIndex === -1) {
			focusedKeyIndex = 0;
			focusKey(focusedKeyIndex);
			return;
		}

		switch (event.key) {
			case "ArrowRight":
				event.preventDefault();
				if (focusedKeyIndex < device.rows * device.columns - 1) {
					// Stay within keypad, move to next key in row
					const col = focusedKeyIndex % device.columns;
					if (col < device.columns - 1) {
						focusedKeyIndex = focusedKeyIndex + 1;
					}
				} else if (focusedKeyIndex < allKeyContexts.length - 1 && device.encoders > 0) {
					// Move to encoders if available
					focusedKeyIndex = device.rows * device.columns;
				}
				focusKey(focusedKeyIndex);
				break;
			case "ArrowLeft":
				event.preventDefault();
				if (focusedKeyIndex >= device.rows * device.columns) {
					// Moving from encoders back to keypad
					if (focusedKeyIndex > device.rows * device.columns) {
						focusedKeyIndex = focusedKeyIndex - 1;
					} else {
						focusedKeyIndex = device.rows * device.columns - 1;
					}
				} else if (focusedKeyIndex > 0) {
					// Stay within keypad, move to previous key in row
					const col = focusedKeyIndex % device.columns;
					if (col > 0) {
						focusedKeyIndex = focusedKeyIndex - 1;
					}
				}
				focusKey(focusedKeyIndex);
				break;
			case "ArrowDown":
				event.preventDefault();
				if (focusedKeyIndex < device.rows * device.columns) {
					// Within keypad, move down a row
					let nextRow = focusedKeyIndex + device.columns;
					if (nextRow < device.rows * device.columns) {
						focusedKeyIndex = nextRow;
						focusKey(focusedKeyIndex);
					}
				}
				break;
			case "ArrowUp":
				event.preventDefault();
				if (focusedKeyIndex < device.rows * device.columns) {
					// Within keypad, move up a row
					let prevRow = focusedKeyIndex - device.columns;
					if (prevRow >= 0) {
						focusedKeyIndex = prevRow;
						focusKey(focusedKeyIndex);
					}
				}
				break;
		}
	}

	function focusKey(index: number) {
		if (index >= 0 && index < allKeyContexts.length) {
			const context = allKeyContexts[index];
			// Update focused key index for visual styling
			focusedKeyIndex = index;
			// Update aria-activedescendant for screen readers
			const keyId = `key-${context.device}-${context.profile}-${context.controller}-${context.position}`;
			const gridElement = document.querySelector('[role="grid"]');
			if (gridElement) {
				gridElement.setAttribute("aria-activedescendant", keyId);
			}
			// Announce to screen reader
			const currentSlot = (context.controller === "Encoder" ? profile.sliders : profile.keys)[context.position];
			const actionName = currentSlot?.action?.name || "Empty";
			announceToScreenReader(`${context.controller === "Encoder" ? "Encoder" : "Key"} ${context.position + 1}: ${actionName}`);
		}
	}

	async function handleAssignAction(action: any, context: Context) {
		if (!action || !context) return;

		let array = context.controller == "Encoder" ? profile.sliders : profile.keys;
		if (array[context.position]) {
			announceToScreenReader("Key already has an action. Delete the current action first.");
			return;
		}

		try {
			array[context.position] = await invoke("create_instance", { context, action });
			profile = profile;
			announceToScreenReader(`${action.name} assigned to ${context.controller} ${context.position + 1}`);

			// Clear the selected action after successful assignment
			selectedAction = null;
			// Dispatch event to notify parent to clear action list selection
			dispatch("actionCleared");
		} catch (error) {
			console.error("Failed to assign action:", error);
			announceToScreenReader("Failed to assign action. Please try again.");
		}
	}

	// Export functions for parent component coordination
	export function handleActionSelected(event: CustomEvent) {
		selectedAction = event.detail;
	}

	export function handleActionCleared() {
		selectedAction = null;
	}

	// Handle request for selected action from Key component
	function handleRequestSelectedAction(event: CustomEvent) {
		if (selectedAction && event.detail.context) {
			handleAssignAction(selectedAction, event.detail.context);
		} else {
			announceToScreenReader("No action selected. Select an action from the action list first.");
		}
	}

	// Handle key actions (Enter, Delete, etc.) on the focused key
	async function handleKeyAction(event: KeyboardEvent) {
		if (focusedKeyIndex < 0 || focusedKeyIndex >= allKeyContexts.length) {
			return;
		}

		const context = allKeyContexts[focusedKeyIndex];
		const currentSlot = (context.controller === "Encoder" ? profile.sliders : profile.keys)[context.position];

		switch (event.key) {
			case "Enter":
			case " ":
				event.preventDefault();
				if (!currentSlot) {
					// Key is empty - try to assign selected action
					if (selectedAction) {
						handleAssignAction(selectedAction, context);
					} else {
						announceToScreenReader("No action selected. Select an action from the action list first.");
					}
				} else {
					// If key has action, select it for property inspection
					inspectedInstance.set(currentSlot.context);
					announceToScreenReader(`Selected ${currentSlot.action?.name || "action"} for editing.`);
				}
				break;
			case "Delete":
			case "Backspace":
				event.preventDefault();
				if (currentSlot) {
					await invoke("remove_instance", { context: currentSlot.context });
					if (inspectedInstance && $inspectedInstance == currentSlot.context) {
						inspectedInstance.set(null);
					}
					// Update the profile to reflect the removal
					(context.controller === "Encoder" ? profile.sliders : profile.keys)[context.position] = null;
					profile = profile;
					announceToScreenReader("Action removed from key.");
				}
				break;
			case "c":
				if (event.ctrlKey && currentSlot) {
					event.preventDefault();
					copiedContext.set(context);
					announceToScreenReader("Action copied to clipboard.");
				}
				break;
			case "v":
				if (event.ctrlKey) {
					event.preventDefault();
					if ($copiedContext && handlePaste) {
						handlePaste($copiedContext, context);
						announceToScreenReader("Action pasted.");
					}
				}
				break;
		}
	}

	// Handle keyboard navigation within the grid
	function handleGridKeyDown(event: KeyboardEvent) {
		// Handle arrow keys for navigation
		if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(event.key)) {
			handleDeviceKeyDown(event);
		} else if (["Enter", " ", "Delete", "Backspace"].includes(event.key) || (event.ctrlKey && (event.key === "c" || event.key === "v"))) {
			// Forward action keys to the focused key
			handleKeyAction(event);
		}
	}
</script>

{#key device}
	<!-- Screen reader announcements -->
	<div bind:this={announcementEl} class="sr-only" aria-live="polite" aria-atomic="true"></div>

	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div
		class="flex flex-col"
		class:hidden={$inspectedParentAction || selectedDevice != device.id}
		role="grid"
		tabindex="0"
		aria-label={`${device.name} Stream Deck with ${device.rows * device.columns} keys${
			device.encoders > 0 ? ` and ${device.encoders} encoders` : ""
		}. Use arrow keys to navigate, Enter to select or assign actions.`}
		on:click={() => inspectedInstance.set(null)}
		on:keydown={handleGridKeyDown}
	>
		<div class="flex flex-col" role="rowgroup">
			{#each { length: device.rows } as _, r}
				<div class="flex flex-row" role="row">
					{#each { length: device.columns } as _, c}
						{@const keyIndex = (r * device.columns) + c}
						<Key
							context={{ device: device.id, profile: profile.id, controller: "Keypad", position: keyIndex }}
							bind:inslot={profile.keys[keyIndex]}
							focused={focusedKeyIndex === keyIndex}
							on:dragover={handleDragOver}
							on:drop={(event) => handleDrop(event, "Keypad", keyIndex)}
							on:dragstart={(event) => handleDragStart(event, "Keypad", keyIndex)}
							on:requestSelectedAction={handleRequestSelectedAction}
							{handlePaste}
							size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
						/>
					{/each}
				</div>
			{/each}
		</div>

		<div class="flex flex-row" role="rowgroup">
			{#each { length: device.encoders } as _, i}
				{@const encoderIndex = (device.rows * device.columns) + i}
				<Key
					context={{ device: device.id, profile: profile.id, controller: "Encoder", position: i }}
					bind:inslot={profile.sliders[i]}
					focused={focusedKeyIndex === encoderIndex}
					on:dragover={handleDragOver}
					on:drop={(event) => handleDrop(event, "Encoder", i)}
					on:dragstart={(event) => handleDragStart(event, "Encoder", i)}
					on:requestSelectedAction={handleRequestSelectedAction}
					{handlePaste}
					size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
				/>
			{/each}
		</div>
	</div>
{/key}

<style>
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		border: 0;
	}
</style>
