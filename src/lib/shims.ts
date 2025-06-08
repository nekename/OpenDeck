import { invoke } from "@tauri-apps/api/core";

globalThis.open = (url?: string | URL) => {
	if (url) invoke("open_url", { url });
	return null;
};
