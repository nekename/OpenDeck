<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";

	import { renderImage, resizeImage } from "$lib/rendererHelper";

	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";

	export let instance: ActionInstance;
	export let showEditor: boolean;

	let state: number = 0;
	let bold: boolean;
	let italic: boolean;

	let fonts: string[] = [];
	onMount(async () => {
		fonts = await invoke("get_fonts");
	});

	let fileInput: HTMLInputElement;
	let solidColourInput: HTMLInputElement;
	let backgroundColourInput: HTMLInputElement;

	function adjustImageScale(delta: number) {
		const next = (instance.states[state].image_scale || 100) + delta;
		instance.states[state].image_scale = Math.max(10, Math.min(200, next));
	}

	function handleDrop(event: DragEvent) {
		event.preventDefault();

		const file = event.dataTransfer?.files?.[0];
		if (!file || !file.type.startsWith("image/")) return;
		const reader = new FileReader();

		reader.onload = async () => {
			let result = reader.result?.toString();
			if (result) {
				let resized = await resizeImage(result);
				if (resized) instance.states[state].image = resized;
				else instance.states[state].image = result;
			}
		};

		reader.readAsDataURL(file);
	}

	function update(instance: ActionInstance) {
		bold = instance.states[state].style.includes("Bold");
		italic = instance.states[state].style.includes("Italic");
	}
	$: update(instance);
	$: invoke("set_state", { context: instance.context, index: state, state: instance.states[state] });
</script>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") showEditor = false;
	}}
/>

<div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 p-2 text-neutral-300 bg-neutral-700 border border-neutral-600 rounded-lg z-10">
	<div class="flex flex-row">
		<div class="select-wrapper m-1 w-full">
			<select class="w-full bg-neutral-600! border-neutral-500!" bind:value={state} aria-label="State">
				{#each instance.states as _, i}
					<option value={i}>State {i + 1}</option>
				{/each}
			</select>
		</div>
		<button class="ml-2 mr-1 float-right text-xl text-neutral-300" on:click={() => showEditor = false} aria-label="Close">✕</button>
	</div>
	<div class="flex flex-row mx-1">
		<div class="flex flex-col justify-center items-center mt-2 mb-1">
			<button
				on:click={(event) => {
					if (event.ctrlKey) return;
					fileInput.click();
				}}
				on:dragover={(event) => {
					event.preventDefault();
					if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
				}}
				on:drop={handleDrop}
				on:contextmenu={(event) => {
					event.preventDefault();
					instance.states[state].image = instance.action.states[state]?.image ?? instance.action.icon;
				}}
				title="Click to select an image, or right-click to reset to the default image."
				aria-label="Click to select an image, or right-click to reset to the default image."
			>
				{#await renderImage(null, null, instance.states[state], instance.action.states[state]?.image ?? instance.action.icon, false, false, true, false, false, 0, true) then resolvedSrc}
					{#if typeof resolvedSrc === "string"}
						<img
							src={resolvedSrc}
							class="my-auto w-32 min-w-32 h-min aspect-square bg-black border border-neutral-600 rounded-xl cursor-pointer"
							alt="State {state + 1} image"
						/>
					{/if}
				{/await}
			</button>
			<div class="flex flex-row items-center justify-center mt-1 space-x-1 text-neutral-300">
				<button
					on:click={() => adjustImageScale(-10)}
					class="w-6 h-6 text-sm bg-neutral-600 hover:bg-neutral-500 transition-colors border border-neutral-500 rounded-md"
					title="Decrease image scale"
					aria-label="Decrease image scale"
				>
					-
				</button>
				<span class="min-w-12 text-center text-xs tabular-nums">
					{instance.states[state].image_scale || 100}%
				</span>
				<button
					on:click={() => adjustImageScale(10)}
					class="w-6 h-6 text-sm bg-neutral-600 hover:bg-neutral-500 transition-colors border border-neutral-500 rounded-md"
					title="Increase image scale"
					aria-label="Increase image scale"
				>
					+
				</button>
			</div>
			<button
				on:click={() => backgroundColourInput.click()}
				on:focus={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) backgroundColourInput.className = "";
				}}
				on:mouseover={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) backgroundColourInput.className = "";
				}}
				on:blur={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) backgroundColourInput.className = "absolute invisible w-0 h-0";
				}}
				on:mouseleave={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) backgroundColourInput.className = "absolute invisible w-0 h-0";
				}}
				class="mt-1 px-0.5 text-sm text-neutral-300 bg-neutral-600 hover:bg-neutral-500 transition-colors border border-neutral-500 rounded-lg"
			>
				Set background
				<input
					bind:this={backgroundColourInput}
					type="color"
					bind:value={instance.states[state].background_colour}
					class="absolute invisible w-0 h-0"
				/>
			</button>
			<button
				on:click={() => solidColourInput.click()}
				on:focus={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) solidColourInput.className = "";
				}}
				on:mouseover={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) solidColourInput.className = "";
				}}
				on:blur={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) solidColourInput.className = "absolute invisible w-0 h-0";
				}}
				on:mouseleave={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) solidColourInput.className = "absolute invisible w-0 h-0";
				}}
				class="mt-1 px-0.5 text-sm text-neutral-300 bg-neutral-600 hover:bg-neutral-500 transition-colors border border-neutral-500 rounded-lg"
			>
				Use solid colour
				<input
					bind:this={solidColourInput}
					type="color"
					class="absolute invisible w-0 h-0"
					value="#FFFFFE"
					on:change={() => {
						const canvas = document.createElement("canvas");
						canvas.width = 1;
						canvas.height = 1;
						const context = canvas.getContext("2d");
						if (!context) return;
						context.fillStyle = solidColourInput.value;
						context.fillRect(0, 0, canvas.width, canvas.height);
						instance.states[state].image = canvas.toDataURL("image/png");
					}}
				/>
			</button>
		</div>
		<input
			bind:this={fileInput}
			type="file"
			class="hidden"
			accept="image/*"
			on:change={async () => {
				if (!fileInput.files || fileInput.files.length == 0) return;
				const reader = new FileReader();

				reader.onload = async () => {
					let result = reader.result?.toString();
					if (result) {
						let resized = await resizeImage(result);
						if (resized) instance.states[state].image = resized;
						else instance.states[state].image = result;
					}
				};

				reader.readAsDataURL(fileInput.files[0]);
			}}
		/>

		<div class="flex flex-col justify-center pl-4 pr-2 pt-4 pb-2 space-y-2">
			<div class="flex flex-row items-center space-x-2">
				<label for="editor-text">Text</label>
				<textarea
					bind:value={instance.states[state].text}
					placeholder={instance.action.states[state]?.text || instance.action.name}
					rows="1"
					class="w-full px-1 text-neutral-300 bg-neutral-600 border border-neutral-500 rounded-lg resize-none"
					id="editor-text"
				/>
			</div>
			<div class="flex flex-row items-center">
				<label for="editor-colour" class="mr-2">Colour</label>
				<input
					type="color"
					bind:value={instance.states[state].colour}
					class="mr-2 px-0.5 bg-neutral-600 border border-neutral-500 rounded-lg"
					id="editor-colour"
				/>
				<label for="editor-show" class="mr-2">Show</label>
				<input
					type="checkbox"
					bind:checked={instance.states[state].show}
					class="mr-4 mt-1 scale-125"
					id="editor-show"
				/>
				<select
					bind:value={instance.states[state].alignment}
					class="px-1! py-0.5!"
					aria-label="Alignment"
				>
					<option value="top">Top</option>
					<option value="middle">Middle</option>
					<option value="bottom">Bottom</option>
				</select>
			</div>
			<div class="flex flex-row items-center">
				<label for="editor-stroke" class="mr-2">Stroke</label>
				<input
					type="color"
					bind:value={instance.states[state].stroke_colour}
					class="mr-2 px-0.5 bg-neutral-600 border border-neutral-500 rounded-lg"
					id="editor-stroke"
				/>
				<label for="editor-outline" class="mr-2">Outline</label>
				<input
					type="number"
					bind:value={instance.states[state].stroke_size}
					class="px-0.5 w-14 text-neutral-300 bg-neutral-600 border border-neutral-500 rounded-lg"
					id="editor-outline"
				/>
			</div>
			<div class="flex flex-row items-center">
				<label for="editor-font" class="mr-2">Font</label>
				<input
					list="families"
					bind:value={instance.states[state].family}
					placeholder="Font family"
					class="w-full px-1 text-neutral-300 bg-neutral-600 border border-neutral-500 rounded-lg"
					id="editor-font"
				/>
				<datalist id="families">
					<option value="Liberation Sans">Liberation Sans</option>
					<option value="Archivo Black">Archivo Black</option>
					<option value="Comic Neue">Comic Neue</option>
					<option value="Courier Prime">Courier Prime</option>
					<option value="Tinos">Tinos</option>
					<option value="Anton">Anton</option>
					<option value="Liberation Serif">Liberation Serif</option>
					<option value="Open Sans">Open Sans</option>
					<option value="Fira Sans">Fira Sans</option>
					<option disabled>──────────</option>
					{#each fonts as font}
						<option value={font}>{font}</option>
					{/each}
				</datalist>
			</div>
			<div class="flex flex-row items-center">
				<label for="editor-bold" class="mr-3 font-bold">B</label>
				<input
					type="checkbox"
					bind:checked={bold}
					on:change={() => instance.states[state].style = bold && italic ? "Bold Italic" : bold ? "Bold" : italic ? "Italic" : "Regular"}
					class="mr-4 mt-1 scale-125"
					id="editor-bold"
				/>
				<label for="editor-italic" class="mr-3 italic">I</label>
				<input
					type="checkbox"
					bind:checked={italic}
					on:change={() => instance.states[state].style = bold && italic ? "Bold Italic" : bold ? "Bold" : italic ? "Italic" : "Regular"}
					class="mr-4 mt-1 scale-125"
					id="editor-italic"
				/>
				<label for="editor-underline" class="mr-3 underline">U</label>
				<input
					type="checkbox"
					bind:checked={instance.states[state].underline}
					class="mr-4 mt-1 scale-125"
					id="editor-underline"
				/>
				<label for="editor-size" class="mr-2">Size</label>
				<input
					type="number"
					bind:value={instance.states[state].size}
					class="px-0.5 w-14 text-neutral-300 bg-neutral-600 border border-neutral-500 rounded-lg"
					id="editor-size"
				/>
			</div>
		</div>
	</div>
</div>
