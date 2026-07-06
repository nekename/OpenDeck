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

	onDestroy(() => previousFocus?.focus());
</script>

{#if show}
	<div
		bind:this={popupEl}
		class="absolute top-0 left-0 m-2 p-4 w-[calc(100%-1rem)] h-[calc(100%-1rem)] bg-neutral-800 border border-neutral-700 rounded-lg overflow-hidden flex flex-col z-30"
		role="dialog"
		tabindex="-1"
		aria-label={label}
	>
		{#if $$slots.header}
			<div class="shrink-0">
				<slot name="header" />
			</div>
		{/if}
		<div class="flex-1 min-h-0 overflow-auto">
			<slot />
		</div>
		{#if $$slots.footer}
			<div class="shrink-0">
				<slot name="footer" />
			</div>
		{/if}
	</div>
{/if}
