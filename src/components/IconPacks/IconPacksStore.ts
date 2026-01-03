import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import type { IconPack } from "$lib/IconPacks";

export const createIconPacksStore = () => {
    const { subscribe, set, update } = writable<{
        data: IconPack[] | null;
        loading: boolean;
        error: unknown | null;
    }>({
        data: null,
        loading: false,
        error: null,
    })

    async function fetch() {
        update(state => ({ ...state, loading: true, error: null }));
        try {
            const response = await invoke("list_installed_iconpacks");
            set({ data: response as IconPack[], loading: false, error: null });
        } catch (error) {
            set({ data: null, loading: false, error: error });
        }
    }

    return {
        subscribe,
        fetch,
        dropById: (id: string) => {
            update(state => {
                if (!state.data) return state;
                return {
                    ...state,
                    data: state.data.filter(pack => pack.id !== id)
                };
            });
        }
    };
};

export const iconPacks = createIconPacksStore();
