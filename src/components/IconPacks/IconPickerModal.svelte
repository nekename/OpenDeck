<script lang="ts">
    import IconPicker from './IconPicker.svelte';
    import Modal from './Modal.svelte';
    import type { IconResult } from "./types";

    let open: boolean = false;
    let onSelect: ((icon: IconResult) => void) | undefined = undefined;

    export { open, onSelect };
</script>

<Modal bind:open class="w-[80vw]">
    <h2 slot="header" class="font-semibold text-xl dark:text-neutral-300">
        Pick an Icon
    </h2>

    <div slot="children" class="mt-4">
        <IconPicker bind:onSelect />
    </div>
</Modal>
