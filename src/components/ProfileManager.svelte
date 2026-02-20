<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import Browsers from "phosphor-svelte/lib/Browsers";
	import FloppyDisk from "phosphor-svelte/lib/FloppyDisk";
	import Pencil from "phosphor-svelte/lib/Pencil";
	import Trash from "phosphor-svelte/lib/Trash";
	import Popup from "./Popup.svelte";

	import { inspectedInstance } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { message } from "@tauri-apps/plugin-dialog";

	let folders: { [name: string]: string[] } = {};
	let value: string;
	async function getProfiles(device: DeviceInfo) {
		let profiles: string[] = await invoke("get_profiles", { device: device.id });
		folders = {};
		for (const id of profiles) {
			let folder = id.includes("/") ? id.split("/")[0] : "";
			if (folders[folder]) folders[folder].push(id);
			else folders[folder] = [id];
		}
		profile = await invoke("get_selected_profile", { device: device.id });
		value = profile.id;
		oldValue = value;
	}

	export let device: DeviceInfo;
	getProfiles(device);

	export let profile: Profile;
	export async function setProfile(id: string) {
		if (!device || !id) return;
		if (value != id) {
			value = id;
			return;
		}
		await invoke("set_selected_profile", { device: device.id, id });
		profile = await invoke("get_selected_profile", { device: device.id });

		let folder = id.includes("/") ? id.split("/")[0] : "";
		if (folders[folder]) {
			if (!folders[folder].includes(id)) folders[folder].push(id);
		} else folders[folder] = [id];
		folders = folders;

		$inspectedInstance = null;
	}

	listen("rerender_images", async () => {
		try {
			profile = await invoke("get_selected_profile", { device: device.id });
		} catch {}
	});

	async function deleteProfile(id: string) {
		for (const devices of Object.values(applicationProfiles)) {
			if (devices[device.id] == id) {
				delete devices[device.id];
				applicationProfiles = applicationProfiles;
			}
		}
		await invoke("delete_profile", { device: device.id, profile: id });
		let folder = id.includes("/") ? id.split("/")[0] : "";
		folders[folder].splice(folders[folder].indexOf(id), 1);
		folders = folders;
	}

	let renamingProfile: string | null = null;
	let renameInput: HTMLInputElement;
	let newId: string = "";

	async function saveRenamedProfile(oldId: string) {
		if (!renameInput.checkValidity() || !newId) return;
		if (newId == oldId) {
			renamingProfile = null;
			return;
		}

		// Check if a profile with the new ID already exists
		const allProfiles = Object.values(folders).flat();
		if (allProfiles.includes(newId)) {
			message(`A profile with the ID "${newId}" already exists.`, { title: "Failed to rename profile" });
			return;
		}

		try {
			await invoke("rename_profile", { device: device.id, oldId, newId });
		} catch (error: any) {
			message(error, { title: "Failed to rename profile" });
			console.error(error);
		}

		// Update application profile mappings
		for (const devices of Object.values(applicationProfiles)) {
			if (devices[device.id] == oldId) devices[device.id] = newId;
		}
		applicationProfiles = applicationProfiles;

		// Update folders structure
		const oldFolder = oldId.includes("/") ? oldId.split("/")[0] : "";
		const newFolder = newId.includes("/") ? newId.split("/")[0] : "";

		// Remove from old folder
		if (folders[oldFolder]) {
			const index = folders[oldFolder].indexOf(oldId);
			if (index != -1) {
				folders[oldFolder].splice(index, 1);
				if (folders[oldFolder].length == 0 && oldFolder != "") delete folders[oldFolder];
			}
		}

		// Add to new folder
		if (folders[newFolder]) folders[newFolder].push(newId);
		else folders[newFolder] = [newId];

		folders = folders;
		renamingProfile = null;
	}
	$: if (renameInput) renameInput.focus();

	let oldValue: string;
	$: {
		if (value == "opendeck_edit_profiles") {
			if (oldValue) showPopup = true;
			value = oldValue;
		} else if (value && value != oldValue && (!profile || profile.id != value)) {
			setProfile(value);
			oldValue = value;
		}
	}

	let showPopup: boolean = false;
	let nameInput: HTMLInputElement;

	let showApplicationManager: boolean = false;
	let applications: string[];
	let applicationProfiles: { [appName: string]: { [device: string]: string } };
	(async () => {
		applications = await invoke("get_applications");
		applicationProfiles = await invoke("get_application_profiles");
	})();
	listen("applications", ({ payload }: { payload: string[] }) => applications = payload);
	let applicationsAddAppName: string = "opendeck_select_application";
	let applicationsAddProfile: string = "opendeck_select_profile";
	$: {
		if (applicationsAddAppName != "opendeck_select_application" && applicationsAddProfile != "opendeck_select_profile") {
			applicationProfiles[applicationsAddAppName] ||= {};
			applicationProfiles[applicationsAddAppName][device.id] = applicationsAddProfile;
			applicationsAddAppName = "opendeck_select_application";
			applicationsAddProfile = "opendeck_select_profile";
		}
	}
	$: {
		if (applicationProfiles) {
			applicationProfiles = Object.fromEntries(Object.entries(applicationProfiles).filter(([_, devices]) => Object.values(devices).filter((v) => v).length != 0));
			invoke("set_application_profiles", { value: applicationProfiles });
		}
	}

	let measure: HTMLSpanElement;
	let selectWidth = 0;
	$: if (value && measure) {
		measure.textContent = value.includes("/") ? value.split("/")[1] : value;
		selectWidth = measure.offsetWidth + 18;
	}
</script>

<div class="select-profile-wrapper">
	<span bind:this={measure} class="invisible fixed whitespace-pre pointer-events-none" aria-hidden="true"></span>
	<select bind:value style:width="{selectWidth}px">
		{#each Object.entries(folders).sort() as [id, profiles]}
			{#if id && profiles.length}
				<optgroup label={id}>
					{#each profiles.sort() as profile}
						<option value={profile}>{profile.split("/")[1]}</option>
					{/each}
				</optgroup>
			{:else}
				{#each profiles.sort() as profile}
					<option value={profile}>{profile}</option>
				{/each}
			{/if}
		{/each}
		<option value="opendeck_edit_profiles">Edit...</option>
	</select>
</div>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") {
			if (showApplicationManager) showApplicationManager = false;
			else if (renamingProfile) renamingProfile = null;
			else showPopup = false;
		}
	}}
/>

<Popup show={showPopup}>
	<button class="mr-1 float-right text-xl text-neutral-300" on:click={() => showPopup = false}>✕</button>
	<h2 class="text-xl font-semibold text-neutral-300">{device.name}</h2>

	<div class="flex flex-row mt-2 mb-1">
		<input
			bind:this={nameInput}
			pattern="[a-zA-Z0-9_ ]+(\/[a-zA-Z0-9_ ]+)?"
			class="grow p-2 text-neutral-300 invalid:text-red-400 bg-neutral-700 border-l border-y border-neutral-600 rounded-l-lg outline-hidden"
			placeholder='Profile name or "folder/name"'
		/>

		<button
			on:click={async () => {
				if (!nameInput.checkValidity() || !nameInput.value) return;
				await setProfile(nameInput.value);
				value = nameInput.value;
				nameInput.value = "";
				showPopup = false;
			}}
			class="px-4 text-neutral-300 bg-neutral-900 hover:bg-neutral-800 transition-colors border-r border-y border-neutral-600 rounded-r-lg"
		>
			Create
		</button>

		<button
			class="ml-2 px-4 flex items-center text-neutral-300 bg-neutral-900 hover:bg-neutral-800 transition-colors border border-neutral-600 rounded-lg outline-hidden"
			on:click={() => showApplicationManager = true}
		>
			<Browsers size={24} />
		</button>
	</div>

	<div class="divide-y divide-neutral-500!">
		{#each Object.entries(folders).sort() as [id, profiles]}
			{#if id && profiles.length}
				<h4 class="py-2 font-bold text-lg text-neutral-300">{id}</h4>
			{/if}
			{#each profiles.sort() as profile}
				<div class="flex flex-row items-center py-2 space-x-2" class:ml-6={id} class:pl-2={id}>
					<input type="radio" bind:group={value} value={profile} disabled={renamingProfile == profile} />
					{#if profile == renamingProfile}
						<input
							bind:this={renameInput}
							bind:value={newId}
							pattern="[a-zA-Z0-9_ ]+(\/[a-zA-Z0-9_ ]+)?"
							class="grow px-2 py-1 text-neutral-300 invalid:text-red-400 bg-neutral-700 rounded outline-hidden"
							placeholder='Profile name or "folder/name"'
							on:keydown={(e) => {
								if (e.key === "Enter") saveRenamedProfile(profile);
							}}
						/>
						<button on:click={() => saveRenamedProfile(profile)} title="Save">
							<FloppyDisk size="20" class="text-green-500" />
						</button>
					{:else}
						<span class="grow text-neutral-400">{id ? profile.split("/")[1] : profile}</span>
						{#if profile != value}
							<button
								on:click={() => renamingProfile = newId = profile}
								title="Rename"
							>
								<Pencil size="20" class="text-neutral-400" />
							</button>
							<button on:click={() => deleteProfile(profile)} title="Delete">
								<Trash size="20" class="text-neutral-400" />
							</button>
						{/if}
					{/if}
				</div>
			{/each}
		{/each}
	</div>
</Popup>

<Popup show={showApplicationManager}>
	<button class="mr-1 float-right text-xl text-neutral-300" on:click={() => showApplicationManager = false}>✕</button>
	<h2 class="text-xl font-semibold text-neutral-300">{device.name}</h2>
	<span class="text-sm text-neutral-400">If your application isn't listed, try switching to it and back again.</span>
	<span class="text-sm text-neutral-400">The 'default profile' will activate when the focussed application has no profile associated with it.</span>

	<table class="w-full text-neutral-300 divide-y divide-neutral-500!">
		{#each Object.entries(applicationProfiles).sort((a, b) => a[0] == "opendeck_default" ? -1 : b[0] == "opendeck_default" ? 1 : a[0].localeCompare(b[0])) as [appName, devices]}
			{#if devices[device.id]}
				<tr class="h-12">
					<td>{appName == "opendeck_default" ? "Default profile" : appName}:</td>
					<td class="select-wrapper">
						<select bind:value={applicationProfiles[appName][device.id]} class="w-full">
							{#each Object.entries(folders) as [id, profiles]}
								{#if id && profiles.length}
									<optgroup label={id}>
										{#each profiles as profile}
											<option value={profile}>{profile.split("/")[1]}</option>
										{/each}
									</optgroup>
								{:else}
									{#each profiles as profile}
										<option value={profile}>{profile}</option>
									{/each}
								{/if}
							{/each}
							<option disabled>──────────</option>
							<option value={undefined}>Remove application</option>
						</select>
					</td>
				</tr>
			{/if}
		{/each}
		<tr class="h-12">
			<td class="w-48 select-wrapper">
				<select bind:value={applicationsAddAppName} class="w-full">
					<option selected disabled value="opendeck_select_application">Select application...</option>
					{#if !applicationProfiles["opendeck_default"] || !applicationProfiles["opendeck_default"][device.id]}
						<option value="opendeck_default">Default profile</option>
						{#if applications.filter((appName) => !applicationProfiles[appName] || !applicationProfiles[appName][device.id]).length > 0}
							<option disabled>──────────</option>
						{/if}
					{/if}
					{#each applications as appName}
						{#if !applicationProfiles[appName] || !applicationProfiles[appName][device.id]}
							<option value={appName}>{appName}</option>
						{/if}
					{/each}
				</select>
			</td>
			<td class="w-96 select-wrapper">
				<select bind:value={applicationsAddProfile} class="w-full">
					<option selected disabled value="opendeck_select_profile">Select profile...</option>
					{#each Object.entries(folders) as [id, profiles]}
						{#if id && profiles.length}
							<optgroup label={id}>
								{#each profiles as profile}
									<option value={profile}>{profile.split("/")[1]}</option>
								{/each}
							</optgroup>
						{:else}
							{#each profiles as profile}
								<option value={profile}>{profile}</option>
							{/each}
						{/if}
					{/each}
				</select>
			</td>
		</tr>
	</table>
</Popup>
