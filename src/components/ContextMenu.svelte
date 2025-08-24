<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Context } from "$lib/Context";
	
	import Clipboard from "phosphor-svelte/lib/Clipboard";
	import Copy from "phosphor-svelte/lib/Copy";
	import Pencil from "phosphor-svelte/lib/Pencil";
	import Trash from "phosphor-svelte/lib/Trash";
	
	import { createEventDispatcher, onMount, tick } from "svelte";
	
	export let slot: ActionInstance | null;
	// context is used in parent component Key.svelte
	export let context: Context | null; // eslint-disable-line
	export let x: number;
	export let y: number;
	
	const dispatch = createEventDispatcher();
	
	let menuElement: HTMLDivElement;
	let menuItems: HTMLButtonElement[] = [];
	let focusedIndex = 0;
	let previousFocus: HTMLElement | null = null;
	
	onMount(() => {
		// Store the previously focused element
		previousFocus = document.activeElement as HTMLElement;
		
		tick().then(() => {
			// Focus the menu and the first item
			if (menuElement) {
				menuElement.focus();
				if (menuItems[0]) {
					menuItems[0].focus();
				}
			}
		});
		
		// Add global escape handler
		const handleGlobalKeydown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				close();
			}
		};
		
		document.addEventListener("keydown", handleGlobalKeydown);
		
		return () => {
			document.removeEventListener("keydown", handleGlobalKeydown);
			// Restore focus when menu closes
			if (previousFocus) {
				previousFocus.focus();
			}
		};
	});
	
	function close() {
		dispatch("close");
		// Restore focus to the previously focused element
		if (previousFocus) {
			previousFocus.focus();
		}
	}
	
	function handleKeyDown(event: KeyboardEvent) {
		event.stopPropagation();
		
		switch (event.key) {
			case "ArrowDown":
				event.preventDefault();
				focusedIndex = (focusedIndex + 1) % menuItems.length;
				menuItems[focusedIndex]?.focus();
				break;
			case "ArrowUp":
				event.preventDefault();
				focusedIndex = (focusedIndex - 1 + menuItems.length) % menuItems.length;
				menuItems[focusedIndex]?.focus();
				break;
			case "Home":
				event.preventDefault();
				focusedIndex = 0;
				menuItems[focusedIndex]?.focus();
				break;
			case "End":
				event.preventDefault();
				focusedIndex = menuItems.length - 1;
				menuItems[focusedIndex]?.focus();
				break;
			case "Escape":
				event.preventDefault();
				close();
				break;
			case "Tab":
				// Trap focus within menu
				event.preventDefault();
				if (event.shiftKey) {
					focusedIndex = (focusedIndex - 1 + menuItems.length) % menuItems.length;
				} else {
					focusedIndex = (focusedIndex + 1) % menuItems.length;
				}
				menuItems[focusedIndex]?.focus();
				break;
		}
	}
	
	function handleEdit() {
		dispatch("edit");
	}
	
	function handleCopy() {
		dispatch("copy");
	}
	
	function handlePaste() {
		dispatch("paste");
	}
	
	function handleClear() {
		dispatch("clear");
	}
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
<div
	bind:this={menuElement}
	class="absolute text-sm font-semibold w-32 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border-2 dark:border-neutral-600 rounded-lg divide-y z-20"
	style={`left: ${x}px; top: ${y}px;`}
	role="menu"
	aria-label="Context menu"
	tabindex="-1"
	on:keydown={handleKeyDown}
	on:click|stopPropagation
>
	{#if !slot}
		<button
			bind:this={menuItems[0]}
			class="flex flex-row p-2 w-full cursor-pointer items-center hover:bg-neutral-200 dark:hover:bg-neutral-600 focus:bg-neutral-200 dark:focus:bg-neutral-600 outline-none"
			role="menuitem"
			tabindex="-1"
			on:click={handlePaste}
		>
			<Clipboard size="18" color={document.documentElement.classList.contains("dark") ? "#DEDDDA" : "#77767B"} />
			<span class="ml-2"> Paste </span>
		</button>
	{:else}
		<button
			bind:this={menuItems[0]}
			class="flex flex-row p-2 w-full cursor-pointer items-center hover:bg-neutral-200 dark:hover:bg-neutral-600 focus:bg-neutral-200 dark:focus:bg-neutral-600 outline-none"
			role="menuitem"
			tabindex="-1"
			on:click={handleEdit}
		>
			<Pencil size="18" color={document.documentElement.classList.contains("dark") ? "#DEDDDA" : "#77767B"} />
			<span class="ml-2"> Edit </span>
		</button>
		<button
			bind:this={menuItems[1]}
			class="flex flex-row p-2 w-full cursor-pointer items-center hover:bg-neutral-200 dark:hover:bg-neutral-600 focus:bg-neutral-200 dark:focus:bg-neutral-600 outline-none"
			role="menuitem"
			tabindex="-1"
			on:click={handleCopy}
		>
			<Copy size="18" color={document.documentElement.classList.contains("dark") ? "#DEDDDA" : "#77767B"} />
			<span class="ml-2"> Copy </span>
		</button>
		<button
			bind:this={menuItems[2]}
			class="flex flex-row p-2 w-full cursor-pointer items-center hover:bg-neutral-200 dark:hover:bg-neutral-600 focus:bg-neutral-200 dark:focus:bg-neutral-600 outline-none"
			role="menuitem"
			tabindex="-1"
			on:click={handleClear}
		>
			<Trash size="18" color="#F66151" />
			<span class="ml-2"> Delete </span>
		</button>
	{/if}
</div>

<!-- Click overlay to close menu when clicking outside -->
<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class="fixed inset-0 z-10"
	role="presentation"
	on:click={close}
/>