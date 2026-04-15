<script lang="ts">
	import type { Context } from "$lib/Context";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";

	export let context: Context;

	const STEPS = 30;
	const DEGREES_PER_TICK = 360 / STEPS; // 12 degrees

	let showControls = false;
	let dialHeld = false;
	let pressing = false;
	let touchFlash = false;
	let touchHoldFlash = false;
	let dialAngle = 0;
	let rotateFlash: 'left' | 'right' | null = null;
	let rotateInterval: number | undefined;
	let showBindings = false;

	// Listen for physical encoder rotation from the device
	listen("encoder_rotated", ({ payload }: { payload: { device: string; position: number; ticks: number } }) => {
		if (payload.device === context.device && payload.position === context.position) {
			dialAngle += payload.ticks * DEGREES_PER_TICK;
			rotateFlash = payload.ticks < 0 ? 'left' : 'right';
			setTimeout(() => rotateFlash = null, 120);
		}
	});

	// Key binding state (stored per encoder in localStorage)
	let bindings = loadBindings();

	function bindingKey(action: string): string {
		return `encoder-bind-${context.device}-${context.position}-${action}`;
	}

	function loadBindings() {
		return {
			rotateLeft: localStorage.getItem(bindingKey('rotateLeft')) || '',
			rotateRight: localStorage.getItem(bindingKey('rotateRight')) || '',
			dialPress: localStorage.getItem(bindingKey('dialPress')) || '',
			dialHold: localStorage.getItem(bindingKey('dialHold')) || '',
			touchPress: localStorage.getItem(bindingKey('touchPress')) || '',
			touchHold: localStorage.getItem(bindingKey('touchHold')) || '',
		};
	}

	function saveBinding(action: string, key: string) {
		localStorage.setItem(bindingKey(action), key);
		bindings = loadBindings();
	}

	function clearBinding(action: string) {
		localStorage.removeItem(bindingKey(action));
		bindings = loadBindings();
	}

	let listeningFor: string | null = null;

	function startListening(action: string) {
		listeningFor = action;
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (!listeningFor) {
			const keyStr = formatKey(event);
			if (keyStr === bindings.rotateLeft) { event.preventDefault(); rotate(-1); }
			else if (keyStr === bindings.rotateRight) { event.preventDefault(); rotate(1); }
			else if (keyStr === bindings.dialPress) { event.preventDefault(); dialPressAction(); }
			else if (keyStr === bindings.dialHold) { event.preventDefault(); dialHoldToggle(); }
			else if (keyStr === bindings.touchPress) { event.preventDefault(); touchPressAction(); }
			else if (keyStr === bindings.touchHold) { event.preventDefault(); touchHoldAction(); }
			return;
		}
		event.preventDefault();
		event.stopPropagation();
		if (event.key === 'Escape') { listeningFor = null; return; }
		if (['Control', 'Shift', 'Alt', 'Meta'].includes(event.key)) return;
		saveBinding(listeningFor, formatKey(event));
		listeningFor = null;
	}

	function formatKey(event: KeyboardEvent): string {
		let parts: string[] = [];
		if (event.ctrlKey) parts.push('Ctrl');
		if (event.altKey) parts.push('Alt');
		if (event.shiftKey) parts.push('Shift');
		if (event.metaKey) parts.push('Super');
		parts.push(event.key.length === 1 ? event.key.toUpperCase() : event.key);
		return parts.join('+');
	}

	// Dial press: quick press and release
	async function dialPressAction() {
		pressing = true;
		await invoke("trigger_virtual_press", { context });
		setTimeout(() => pressing = false, 150);
	}

	// Dial hold: toggle press down / release
	async function dialHoldToggle() {
		if (dialHeld) {
			dialHeld = false;
			await invoke("trigger_virtual_encoder_up", { context });
		} else {
			dialHeld = true;
			pressing = true;
			await invoke("trigger_virtual_encoder_down", { context });
			setTimeout(() => pressing = false, 150);
		}
	}

	// Touch screen press
	async function touchPressAction() {
		touchFlash = true;
		await invoke("trigger_virtual_touch", { context, hold: false });
		setTimeout(() => touchFlash = false, 150);
	}

	// Touch screen hold
	async function touchHoldAction() {
		touchHoldFlash = true;
		await invoke("trigger_virtual_touch", { context, hold: true });
		setTimeout(() => touchHoldFlash = false, 150);
	}

	async function rotate(ticks: number) {
		rotateFlash = ticks < 0 ? 'left' : 'right';
		// dialAngle is updated by the encoder_rotated event listener,
		// which fires for both physical and virtual rotation
		await invoke("trigger_virtual_rotate", { context, ticks });
		setTimeout(() => rotateFlash = null, 120);
	}

	function startRotate(ticks: number) {
		rotate(ticks);
		rotateInterval = window.setInterval(() => rotate(ticks), 150);
	}

	function stopRotate() {
		if (rotateInterval !== undefined) {
			clearInterval(rotateInterval);
			rotateInterval = undefined;
		}
	}

	function handleDialClick() {
		showControls = !showControls;
		if (!showControls) { showBindings = false; listeningFor = null; }
	}

	function handleClickOutside(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('.encoder-dial-wrapper')) {
			showControls = false;
			showBindings = false;
			listeningFor = null;
		}
	}

	// Generate notch positions around the dial
	function notchTransform(index: number): string {
		const angle = (index / STEPS) * 360;
		return `rotate(${angle}deg)`;
	}
</script>

<svelte:window on:click={handleClickOutside} on:keydown={handleKeyDown} />

<div class="flex-1 flex justify-center pt-2 encoder-dial-wrapper relative">
	<!-- The dial circle with 30 notch marks -->
	<button
		class="w-12 h-12 rounded-full border-2 bg-neutral-800 cursor-pointer focus:outline-none transition-all duration-100 relative overflow-hidden"
		class:border-blue-500={showControls}
		class:border-neutral-600={!showControls && !dialHeld}
		class:border-amber-500={dialHeld}
		style="transform: {pressing ? 'scale(0.9)' : 'scale(1)'};"
		on:click|stopPropagation={handleDialClick}
		on:dblclick|stopPropagation={dialPressAction}
		aria-label="Encoder {context.position + 1} dial"
		title="Click for controls, double-click to press"
	>
		<!-- Rotating notch ring -->
		<div
			class="absolute inset-0 transition-transform duration-75"
			style="transform: rotate({dialAngle}deg);"
		>
			<!-- 30 notch marks around the edge -->
			{#each { length: STEPS } as _, i}
				<div
					class="absolute left-1/2 top-0"
					style="transform: {notchTransform(i)} translateX(-50%); transform-origin: 50% 24px;"
				>
					<div
						class="rounded-full"
						style="width: {i === 0 ? '2.5px' : '1.5px'}; height: {i === 0 ? '6px' : '3px'}; background: {i === 0 ? '#94a3b8' : '#525252'};"
					></div>
				</div>
			{/each}
		</div>
	</button>

	{#if showControls}
		<div
			class="absolute top-16 z-20 flex flex-col items-center gap-2 bg-neutral-800 border border-neutral-600 rounded-lg p-2.5 shadow-lg"
			style="min-width: 190px;"
			on:click|stopPropagation
		>
			<!-- Rotation row -->
			<div class="flex flex-row items-center gap-1.5 w-full justify-center">
				<button
					class="w-9 h-9 rounded-md text-neutral-300 text-lg flex items-center justify-center cursor-pointer transition-all duration-100 select-none"
					class:bg-blue-600={rotateFlash === 'left'}
					class:bg-neutral-700={rotateFlash !== 'left'}
					on:mousedown|stopPropagation={() => startRotate(-1)}
					on:mouseup={stopRotate}
					on:mouseleave={stopRotate}
					title="Rotate left (hold to repeat)"
					aria-label="Rotate encoder left"
				>&#x21BA;</button>

				<span class="text-[10px] text-neutral-500 uppercase tracking-wide w-12 text-center">Rotate</span>

				<button
					class="w-9 h-9 rounded-md text-neutral-300 text-lg flex items-center justify-center cursor-pointer transition-all duration-100 select-none"
					class:bg-blue-600={rotateFlash === 'right'}
					class:bg-neutral-700={rotateFlash !== 'right'}
					on:mousedown|stopPropagation={() => startRotate(1)}
					on:mouseup={stopRotate}
					on:mouseleave={stopRotate}
					title="Rotate right (hold to repeat)"
					aria-label="Rotate encoder right"
				>&#x21BB;</button>
			</div>

			<!-- Dial section label -->
			<span class="text-[10px] text-neutral-500 uppercase tracking-wide">Dial</span>

			<!-- Dial press/hold row -->
			<div class="flex flex-row items-center gap-1.5 w-full justify-center">
				<button
					class="h-8 px-2.5 rounded-md text-neutral-300 text-xs flex items-center gap-1.5 cursor-pointer transition-all duration-100"
					class:bg-blue-600={pressing && !dialHeld}
					class:bg-neutral-700={!pressing || dialHeld}
					on:click|stopPropagation={dialPressAction}
					title="Press dial (quick press and release)"
					aria-label="Press encoder dial"
				>
					<!-- Press icon: finger pushing down -->
					<svg width="12" height="14" viewBox="0 0 12 14" fill="currentColor">
						<circle cx="6" cy="3" r="2.5" fill="none" stroke="currentColor" stroke-width="1.5"/>
						<line x1="6" y1="5.5" x2="6" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
						<path d="M3 9 L6 12 L9 9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
					</svg>
					Press
				</button>

				<button
					class="h-8 px-2.5 rounded-md text-xs flex items-center gap-1.5 cursor-pointer transition-all duration-100"
					class:bg-amber-600={dialHeld}
					class:text-white={dialHeld}
					class:bg-neutral-700={!dialHeld}
					class:text-neutral-300={!dialHeld}
					on:click|stopPropagation={dialHoldToggle}
					title="Hold dial down (click again to release)"
					aria-label="Hold encoder dial"
				>
					<!-- Press icon + lock -->
					<svg width="20" height="14" viewBox="0 0 20 14" fill="currentColor">
						<circle cx="5" cy="3" r="2.5" fill="none" stroke="currentColor" stroke-width="1.5"/>
						<line x1="5" y1="5.5" x2="5" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
						<path d="M2 9 L5 12 L8 9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
						<rect x="12" y="7" width="7" height="5.5" rx="1"/>
						<path d="M13.5 7 V5 A2 2 0 0 1 17.5 5 V7" fill="none" stroke="currentColor" stroke-width="1.3"/>
					</svg>
					Hold
				</button>
			</div>

			<!-- Touch section label -->
			<span class="text-[10px] text-neutral-500 uppercase tracking-wide">Touch Screen</span>

			<!-- Touch press/hold row -->
			<div class="flex flex-row items-center gap-1.5 w-full justify-center">
				<button
					class="h-8 px-2.5 rounded-md text-xs flex items-center gap-1.5 cursor-pointer transition-all duration-100"
					class:bg-blue-600={touchFlash}
					class:text-white={touchFlash}
					class:bg-neutral-700={!touchFlash}
					class:text-neutral-300={!touchFlash}
					on:click|stopPropagation={touchPressAction}
					title="Press the encoder touch screen"
					aria-label="Press encoder screen"
				>
					<!-- Touch/screen tap icon -->
					<svg width="12" height="14" viewBox="0 0 12 14" fill="currentColor">
						<rect x="1" y="0" width="10" height="8" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.2"/>
						<circle cx="6" cy="4" r="1.5"/>
						<line x1="6" y1="8" x2="6" y2="13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
					</svg>
					Press
				</button>

				<button
					class="h-8 px-2.5 rounded-md text-xs flex items-center gap-1.5 cursor-pointer transition-all duration-100"
					class:bg-amber-600={touchHoldFlash}
					class:text-white={touchHoldFlash}
					class:bg-neutral-700={!touchHoldFlash}
					class:text-neutral-300={!touchHoldFlash}
					on:click|stopPropagation={touchHoldAction}
					title="Long press the encoder touch screen"
					aria-label="Long press encoder screen"
				>
					<!-- Touch/screen + lock icon -->
					<svg width="22" height="14" viewBox="0 0 22 14" fill="currentColor">
						<rect x="1" y="0" width="10" height="8" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.2"/>
						<circle cx="6" cy="4" r="1.5"/>
						<line x1="6" y1="8" x2="6" y2="13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
						<rect x="14" y="7" width="7" height="5.5" rx="1"/>
						<path d="M15.5 7 V5 A2 2 0 0 1 19.5 5 V7" fill="none" stroke="currentColor" stroke-width="1.3"/>
					</svg>
					Hold
				</button>
			</div>

			<!-- Keybindings toggle -->
			<button
				class="h-7 px-2 rounded-md text-xs flex items-center gap-1 cursor-pointer transition-colors w-full justify-center border-t border-neutral-700 mt-0.5 pt-1"
				class:bg-blue-600={showBindings}
				class:text-white={showBindings}
				class:bg-neutral-700={!showBindings}
				class:text-neutral-400={!showBindings}
				on:click|stopPropagation={() => { showBindings = !showBindings; listeningFor = null; }}
				title="Assign keyboard shortcuts to each action"
				aria-label="Configure key bindings"
			>
				<svg width="12" height="10" viewBox="0 0 12 10" fill="currentColor">
					<rect x="0" y="0" width="12" height="10" rx="1.5" fill="none" stroke="currentColor" stroke-width="1"/>
					<rect x="2" y="2.5" width="1.5" height="1.5" rx="0.3"/>
					<rect x="4.25" y="2.5" width="1.5" height="1.5" rx="0.3"/>
					<rect x="6.5" y="2.5" width="1.5" height="1.5" rx="0.3"/>
					<rect x="8.5" y="2.5" width="1.5" height="1.5" rx="0.3"/>
					<rect x="2" y="5.5" width="1.5" height="1.5" rx="0.3"/>
					<rect x="4.25" y="5.5" width="3.75" height="1.5" rx="0.3"/>
					<rect x="8.5" y="5.5" width="1.5" height="1.5" rx="0.3"/>
				</svg>
				Key Bindings
			</button>

			<!-- Key bindings panel -->
			{#if showBindings}
				<div class="flex flex-col gap-1.5 w-full border-t border-neutral-600 pt-2">
					{#each [
						{ action: 'rotateLeft', label: 'Rotate L', value: bindings.rotateLeft },
						{ action: 'rotateRight', label: 'Rotate R', value: bindings.rotateRight },
						{ action: 'dialPress', label: 'Dial Press', value: bindings.dialPress },
						{ action: 'dialHold', label: 'Dial Hold', value: bindings.dialHold },
						{ action: 'touchPress', label: 'Touch Press', value: bindings.touchPress },
						{ action: 'touchHold', label: 'Touch Hold', value: bindings.touchHold },
					] as { action, label, value }}
						<div class="flex flex-row items-center gap-1.5">
							<span class="text-xs text-neutral-400 w-18 shrink-0">{label}</span>
							<button
								class="flex-1 h-6 px-1.5 rounded text-xs text-left truncate cursor-pointer transition-colors"
								class:bg-blue-700={listeningFor === action}
								class:text-white={listeningFor === action}
								class:bg-neutral-700={listeningFor !== action}
								class:text-neutral-300={listeningFor !== action && value}
								class:text-neutral-500={listeningFor !== action && !value}
								on:click|stopPropagation={() => startListening(action)}
								title={value || 'Click to bind a key'}
							>
								{#if listeningFor === action}
									Press a key...
								{:else}
									{value || 'None'}
								{/if}
							</button>
							{#if value}
								<button
									class="w-5 h-5 rounded text-neutral-500 hover:text-red-400 bg-neutral-700 hover:bg-neutral-600 text-xs flex items-center justify-center cursor-pointer transition-colors"
									on:click|stopPropagation={() => clearBinding(action)}
									title="Clear binding"
								>&times;</button>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>
