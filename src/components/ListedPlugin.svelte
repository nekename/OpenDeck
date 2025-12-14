<script lang="ts">
	export let icon: string;
	export let name: string;
	export let subtitle: string;
	export let hidden: boolean = false;
	export let disconnected: boolean = false;
	export let action: () => void;
	export let secondaryAction: (() => void) | undefined = undefined;
</script>

<div
	class="flex flex-row items-center m-2 p-2 bg-neutral-200 dark:bg-neutral-700 rounded-md"
	class:hidden
>
	<img src={icon} class="w-24 h-24 rounded-md" class:opacity-75={disconnected} alt={name} loading="lazy" />
	<div class="ml-4 mr-2 dark:text-neutral-300 wrap-anywhere" class:opacity-75={disconnected}>
		<p class="font-semibold">{name}</p>
		<slot name="subtitle">{subtitle}</slot>
	</div>

	<div class="flex flex-col ml-auto mr-4">
		{#if secondaryAction}
			<button on:click={secondaryAction}>
				<slot name="secondary" />
			</button>
		{/if}
		<button on:click={action}>
			<slot />
		</button>
	</div>
</div>
