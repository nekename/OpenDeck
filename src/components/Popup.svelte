<script lang="ts">
	import { onDestroy, tick } from "svelte";

	export let show = false;
	export let label = "";

	let popupEl: HTMLDivElement;
	let previousFocus: HTMLElement | null = null;

	$: if (show) {
		previousFocus = document.activeElement as HTMLElement | null;
		tick().then(() => popupEl?.focus());
	} else if (previousFocus) {
		previousFocus.focus();
		previousFocus = null;
	}

	onDestroy(() => {
		previousFocus?.focus();
	});
</script>

{#if show}
	<div
		bind:this={popupEl}
		class="absolute top-0 left-0 m-2 p-4 w-[calc(100%-1rem)] h-[calc(100%-1rem)] bg-neutral-800 border border-neutral-700 rounded-lg overflow-auto z-30"
		role="dialog"
		aria-label={label}
		tabindex="-1"
	>
		<slot />
	</div>
{/if}
