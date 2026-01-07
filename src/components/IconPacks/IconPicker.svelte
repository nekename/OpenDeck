<script lang="ts">
    import { Asterisk } from "phosphor-svelte";
    import { invoke } from "@tauri-apps/api/core";
    import type { IconResult } from "./types";

    let searchQuery: string = "";
    let searchResults: Array<IconResult> = [];
    let onSelect: ((icon: IconResult) => void) | undefined = undefined;

    export { onSelect };

    function handleSearch() {
        if (searchQuery.trim() === "") {
            searchResults = [];
            return;
        } else if (searchQuery.length < 3) {
            // minimum 3 characters to search
            searchResults = [];
            return;
        }

        // Call the backend to search for icons
        invoke("search_icons", { query: `(?i).*${searchQuery}.*` })
            .then((results) => {
                // top 10 results
                searchResults = (results as IconResult[]).slice(0, 100);
            })
            .catch((error) => {
                console.error("Error searching icons:", error);
                searchResults = [];
            });
    }

    const showAll = () => {
        searchQuery = "";
        invoke("search_icons", { query: `(?i).*${searchQuery}.*` }).then((results) => {
            // top 10 results
            searchResults = results as IconResult[];
        })
        .catch((error) => {
            console.error("Error searching icons:", error);
            searchResults = [];
        });
    }
</script>

<div class="flex flex-col gap-4">
    <div class="flex space-between gap-4">
        <input
            type="text"
            placeholder="Search icons... (min. 3 characters)"
            spellcheck="false"
            class="grow p-2 border border-neutral-300 dark:border-neutral-600 rounded-lg bg-neutral-50 dark:bg-neutral-800 text-neutral-900 dark:text-neutral-100 outline-hidden"
            bind:value={searchQuery}
            on:input={handleSearch}
        />

        <button
            class="flex-none p-1 px-3 text-sm text-neutral-700 dark:text-neutral-300 bg-neutral-100 dark:bg-neutral-700 border dark:border-neutral-600 rounded-lg outline-hidden"
            on:click={showAll}
        >
            <Asterisk />
        </button>
    </div>

    {#if searchResults.length > 0}
        <div class="flex flex-wrap gap-4">
            {#each searchResults as result, i}
                {@const url = `icon://localhost/${result.pack}/${result.name}`}
                <!-- svelte-ignore a11y-click-events-have-key-events -->
                <div
                    class="flex h-12 w-12 items-center justify-center border border-neutral-300 dark:border-neutral-600 rounded-lg cursor-pointer hover:bg-neutral-100 dark:hover:bg-neutral-700"
                    role="button"
                    tabindex="{i + 1}"
                    on:click={() => onSelect && onSelect(result)}
                >
                    <img
                        src={url}
                        alt={result.name}
                        class="max-w-full max-h-full"
                    />
                </div>
            {/each}
        </div>
    {:else if searchQuery.trim() !== "" && searchQuery.length >= 3}
        <p class="text-sm text-neutral-500">
            No icons found for "{searchQuery}"
        </p>
    {/if}
</div>
