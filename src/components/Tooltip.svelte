<script>
	import Question from "phosphor-svelte/lib/Question";

	let show = false;
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class="inline-block cursor-help"
	on:mouseenter={() => show = true}
	on:mouseleave={() => show = false}
>
	<Question size="24" class="text-neutral-500" />
	{#if show}
		<div class="absolute mt-2 p-2 w-96 text-xs dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border-2 dark:border-neutral-600 rounded-lg">
			<slot />
		</div>
	{/if}
</div>
