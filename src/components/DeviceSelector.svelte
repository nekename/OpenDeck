<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import { profileManager } from "$lib/singletons";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { currentMonitor, getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

	export let devices: { [id: string]: DeviceInfo } = {};
	export let value: string;
	export let selectedProfiles: { [id: string]: Profile } = {};

	let registered: string[] = [];
	$: {
		if (!value || !devices[value]) value = Object.keys(devices).sort()[0];
		for (const [id, device] of Object.entries(devices)) {
			if (!registered.includes(id)) {
				(async () => {
					let profile: Profile = await invoke("get_selected_profile", { device: device.id });
					selectedProfiles[id] = profile;
					await invoke("set_selected_profile", { device: id, id: profile.id });
				})();
				registered.push(id);
			}
		}
	}

	export function reloadProfiles() {
		registered = [];
	}

	listen("switch_profile", async ({ payload }: { payload: { device: string; profile: string } }) => {
		if (payload.device == value) {
			$profileManager?.setProfile(payload.profile);
		} else {
			await invoke("set_selected_profile", { device: payload.device, id: payload.profile });
			selectedProfiles[payload.device] = await invoke("get_selected_profile", { device: payload.device });
		}
	});

	(async () => devices = await invoke("get_devices"))();
	listen("devices", ({ payload }: { payload: { [id: string]: DeviceInfo } }) => devices = payload);

	let buildInfo: string;
	(async () => buildInfo = await invoke("get_build_info"))();
	const window = getCurrentWindow();

	$: {
		if (devices[value]) {
			const effectiveCols = Math.min(Math.max(devices[value].columns, devices[value].encoders, devices[value].touchpoints), 8);
			const effectiveRows = Math.min(devices[value].rows + Math.min(devices[value].encoders, 1) + Math.min(devices[value].touchpoints, 1), 4);
			const idealWidth = (effectiveCols * 132) + 416;
			const idealHeight = (effectiveRows * 132) + 384 + (buildInfo?.includes("darwin") ? 28 : 0);
			(async () => {
				const monitor = await currentMonitor();
				const scaleFactor = monitor?.scaleFactor ?? 1;
				// Convert the monitor's work area to logical pixels
				const maxWidth = (monitor?.workArea.size.width ?? Infinity) / scaleFactor;
				const maxHeight = (monitor?.workArea.size.height ?? Infinity) / scaleFactor;
				const width = Math.min(idealWidth, maxWidth);
				const height = Math.min(idealHeight, maxHeight);
				await window.setMinSize(new LogicalSize(width, height));
				await window.setSize(new LogicalSize(width, height));
			})();
		}
	}

	let measure: HTMLSpanElement;
	let selectWidth = 0;
	$: if (value && measure && devices[value]) {
		measure.textContent = devices[value].name;
		selectWidth = measure.offsetWidth + 20;
	}
</script>

{#if Object.keys(devices).length > 0}
	<div class="select-device-wrapper">
		<span bind:this={measure} class="invisible fixed whitespace-pre pointer-events-none text-xl font-semibold" aria-hidden="true"></span>
		<select bind:value style:width="{selectWidth}px">
			<option value="" disabled selected>Choose a device...</option>

			{#each Object.entries(devices).sort() as [id, device]}
				<option value={id}>{device.name}</option>
			{/each}
		</select>
	</div>
{/if}
