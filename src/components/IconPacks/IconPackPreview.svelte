<script lang="ts">
    import { Check, Trash } from "phosphor-svelte";
    import type { IconPack } from "$lib/IconPacks";
    import IconLogo from "./IconLogo.svelte";

    let className: string = "";
    let pack: IconPack | undefined = undefined;
    let installed: boolean = false;
    let onInstall: undefined | (() => Promise<void>) = undefined;
    let onRemove: undefined | (() => Promise<void>) = undefined;

    export { className as class, pack, installed, onInstall, onRemove };
</script>

<div class={`${className} flex flex-row gap-4`}>
    {#if pack}
        {@const { id, name, author, version, icon } = pack}
        <IconLogo {icon} />

        <div class="flex flex-col">
            <div
                class="flex flex-row gap-1 text-neutral-500 dark:text-neutral-400"
            >
                <span>Id:</span>
                <span class="font-semibold">{id}</span>
            </div>

            <div
                class="flex flex-row gap-1 text-neutral-500 dark:text-neutral-400"
            >
                <span>Author:</span>
                <span class="font-semibold">{author}</span>
            </div>

            <div
                class="flex flex-row gap-1 text-neutral-500 dark:text-neutral-400"
            >
                <span>Version:</span>
                <span class="font-semibold">{version}</span>
            </div>

            <span class="flex-grow"></span>

            <div class="flex flex-row gap-2">
                <button
                    class="px-2 py-1 flex gap-1 items-center justify-center text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border dark:border-neutral-600 rounded-lg outline-hidden"
                    on:click={onInstall}
                    disabled={installed || !onInstall}
                >
                    {#if installed}
                        <Check />
                        Installed
                    {:else}
                        Install
                    {/if}
                </button>
                {#if installed && onRemove}
                    <button
                        class="px-2 text-sm text-red-400 dark:text-red-400 bg-neutral-100 dark:bg-neutral-700 border dark:border-neutral-600 rounded-lg outline-hidden"
                        on:click={onRemove}
                    >
                        <Trash />
                    </button>
                {/if}
            </div>
        </div>
    {:else}
        <p class="m-2 text-neutral-500 dark:text-neutral-400">
            No Icon Pack Selected
        </p>
    {/if}
</div>
