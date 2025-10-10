import { invoke } from "@tauri-apps/api/core";

let portBase = 57116;
let initialised = false;

export async function initPortBase(): Promise<void> {
	if (initialised) return;
	portBase = await invoke<number>("get_port_base");
	initialised = true;
}

export function getWebSocketPort(): number {
	return portBase;
}

export function getWebserverUrl(path: string = ""): string {
	return `http://localhost:${portBase + 2}/${path}`;
}
