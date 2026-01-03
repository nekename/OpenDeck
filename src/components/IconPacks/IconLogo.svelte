<script lang="ts">
    import type { IconPackAvatar } from "$lib/IconPacks";
    import { getWebserverUrl } from "$lib/ports";

    let className: string = "";
    let icon: IconPackAvatar | undefined = undefined;

    export { className as class, icon };

    let src =
        icon && icon.type === "dataurl"
            ? icon.url
            : icon?.type === "fs" && icon.path
              ? getWebserverUrl(icon.path)
              : "";
</script>

<div
    class={`${className} w-32 h-32 flex items-center justify-center rounded-lg border dark:border-neutral-600 bg-neutral-200 dark:bg-neutral-700 text-neutral-500 dark:text-neutral-400 overflow-hidden`}
>
    {#if src}
        <img {src} alt="Icon Pack Preview" />
    {:else}
        No Preview
    {/if}
</div>
