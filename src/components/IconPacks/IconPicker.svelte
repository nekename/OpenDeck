<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";

    type IconResult = {
        pack: string;
        name: string;
        file_name: string;
    };

    let searchQuery: string = "";
    let searchResults: Array<IconResult> = [];

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
        invoke("search_icons", { query: `.*${searchQuery}.*` })
            .then((results) => {
                // top 10 results
                searchResults = (results as IconResult[]).slice(0, 10);
            })
            .catch((error) => {
                console.error("Error searching icons:", error);
                searchResults = [];
            });
    }
</script>

<div class="flex flex-col gap-4">
    <input
        type="text"
        placeholder="Search icons..."
        spellcheck="false"
        class="p-2 border border-neutral-300 dark:border-neutral-600 rounded-lg bg-neutral-50 dark:bg-neutral-800 text-neutral-900 dark:text-neutral-100 outline-hidden"
        bind:value={searchQuery}
        on:input={handleSearch}
    />
    {#if searchResults.length > 0}
        <div class="flex gap-4">
            {#each searchResults as result}
                {@const url = `icon://localhost/${result.pack}/${result.file_name}`}
                <div
                    class="flex h-12 w-12 items-center justify-center border border-neutral-300 dark:border-neutral-600 rounded-lg cursor-pointer hover:bg-neutral-100 dark:hover:bg-neutral-700"
                >
                    <img
                        src={url}
                        srcset={url}
                        alt={result.name}
                        class="max-w-full max-h-full"
                    />
                </div>
            {/each}
        </div>
    {:else if searchQuery.trim() !== ""}
        <p class="text-sm text-neutral-500">
            No icons found for "{searchQuery}"
        </p>
    {/if}
</div>
