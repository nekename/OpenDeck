<script lang="ts">
	import type { Action } from "$lib/Action";

	export let action: Action;
	export let localisation: { name: string; tooltip: string };

	function handleDragStart(event: DragEvent) {
		event.dataTransfer?.setData("action", JSON.stringify(action));
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		return true;
	}
</script>

<div class="flex flex-row items-center mt-2 mb-2 space-x-2">
	<img
		src={!action.icon.startsWith("opendeck/") ? "http://localhost:57118/" + action.icon : action.icon.replace("opendeck", "")}
		alt={localisation.tooltip}
		class="w-12 h-12 rounded-xs"
		draggable="true"
		on:dragstart={handleDragStart}
		on:dragover={handleDragOver}
	/>
	<span class="dark:text-neutral-400"> {localisation.name} </span>
</div>
