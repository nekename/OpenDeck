<script lang="ts">
    let open: boolean = false;
    let className: string = "";

    export { open, className as class };

    let dialog: HTMLDialogElement;

    $: if (dialog && open) dialog.showModal();
    $: if (dialog && !open) dialog.close();
</script>

<!-- svelte-ignore a11y-no-noninteractive-element-interactions a11y-click-events-have-key-events -->
<dialog
    class={className}
    bind:this={dialog}
    on:close={() => (open = false)}
    on:click|self={() => dialog.close()}
>
    <div
        on:click|stopPropagation
        role="presentation"
        class="p-4 bg-neutral-100 dark:bg-neutral-800 border-2 dark:border-neutral-700 rounded-lg"
    >
        <div class="flex flex-row justify-between">
            <slot name="header" />
            <button
                class="mr-2 my-1 float-right text-xl dark:text-neutral-300"
                on:click={() => dialog.close()}>✕</button
            >
        </div>

        <slot name="children" />
    </div>
</dialog>

<style>
    dialog {
        background-color: unset;

        /* https://tailwindcss.com/docs/upgrade-guide#dialog-margins-removed */
        margin: auto;
    }

    ::backdrop {
        background: rgba(0, 0, 0, 0.5);
    }
</style>
