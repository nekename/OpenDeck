<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import type { PreviewPackInfo } from "./types";
    import IconPackPreview from "./IconPackPreview.svelte";
    import Modal from "./Modal.svelte";

    let open: boolean = false;
    let pack: PreviewPackInfo | undefined = undefined;
    let isInstalled: boolean = false;
    let onInstall: ((path: string) => Promise<void>) | undefined = undefined;

    export { open, pack, isInstalled, onInstall };
</script>

<Modal bind:open>
    <h2 slot="header" class="m-2 font-semibold text-xl dark:text-neutral-300">
        Preview: {pack?.meta?.name}
    </h2>

    <IconPackPreview
        slot="children"
        pack={pack?.meta}
        installed={isInstalled}
        onInstall={pack &&
            (async () => {
                await invoke("install_sd_iconpack", {
                    path: pack.path,
                });
                if (onInstall) await onInstall(pack.path);
            })}
    />
</Modal>
