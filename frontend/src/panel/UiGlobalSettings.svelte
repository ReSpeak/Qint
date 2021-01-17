<script lang="typescript">
	import { onMount } from "svelte";
	import type { Writable } from "svelte/store";
	import { app } from "../app";
	import BTabList from "../ui/BTabList.svelte";
	import BTabSlot from "../ui/BTabSlot.svelte";
	import BDropDown from "../ui/BDropDown.svelte";
	import BKeyValue from "../ui/BKeyValue.svelte";
	import BHotkeyField from "../ui/BHotkeyField.svelte";
	import SimpleDiagram from "../ui/UiSimpleDiagram.svelte";
	import BSlider from "../ui/BSlider.svelte";
	import { dbToFactor, factorToDb, LOUDNESS_END_MAGIC, LOUDNESS_MAX, LOUDNESS_MIN, LOUDNESS_UPDATE_MS, MIN_VOLUME_DB } from "../util";
	import { hotkeyToShortcut, shortcutToHotkey } from "../hotkey";
	import type { Hotkey } from "../hotkey";
	import Icon from "../ui/Icon.svelte";
	import { backend } from "../backend/backend";

	const shortcuts = app.transientSettings.shortcuts;
	const hotkeyActions = shortcuts.actions;
	let incompleteHotkey: Hotkey | undefined;

	let tablistIndex: Writable<number>;
	const audioSett = app.transientSettings.audio;

	let developMode = app.transientSettings.ui._developMode;
	let minGlobalVolume = MIN_VOLUME_DB;
	let maxGlobalVolume = -MIN_VOLUME_DB;
	let minLoudnessThreshold = -100;
	let maxLoudnessThreshold = 100;
	let browserNotificationDenied = false;

	let globalVolume = factorToDb(audioSett.globalVolume);
	let loudnessThreshold = audioSett.loudnessThreshold ?? minLoudnessThreshold;
	let loudnessDiagram: SimpleDiagram | undefined;
	let loudnessSilenceTimer: number | undefined;

	let loudnessSocket: WebSocket | undefined;
	let loudness: number | undefined;
	const LOUDNESS_WIDTH = 1000;
	const LOUDNESS_HEIGHT = 300;
	const LOUDNESS_COUNT = 500;
	$: if (loudness && loudnessDiagram) {
		loudnessDiagram.addValue(loudness);
		if (loudness === LOUDNESS_END_MAGIC) {
			if (loudnessSilenceTimer === undefined) {
				let counter = LOUDNESS_COUNT;
				loudnessSilenceTimer = setInterval(() => {
					loudnessDiagram?.addValue(LOUDNESS_END_MAGIC);
					counter--;
					if (counter === 0) {
						clearInterval(loudnessSilenceTimer);
						loudnessSilenceTimer = undefined;
					}
				}, LOUDNESS_UPDATE_MS);
			}
		} else if (loudnessSilenceTimer !== undefined) {
			clearInterval(loudnessSilenceTimer);
			loudnessSilenceTimer = undefined;
		}
	}

	// Reload settings
	app.transientSettings.loadAsync().then(() => {
		globalVolume = factorToDb(audioSett.globalVolume);
		loudnessThreshold = audioSett.loudnessThreshold ?? loudnessThreshold;
	});

	// Text-to-Speech
	const synthSett = app.transientSettings.synth;
	const voices = synthSett.getVoices();
	let previewText!: HTMLInputElement;
	function previewVoice() {
		const text = previewText.value;
		synthSett.trySpeak(text);
	}

	function syncSettings() {
		app.transientSettings.save();
	}

	async function createHotkey() {
		if (incompleteHotkey === undefined) {
			incompleteHotkey = {
				keycode: null,
				ctrl: false,
				shift: false,
				alt: false,
				meta: false,
				action: null,
			};
		}
	}

	function changeHotkey(e: CustomEvent<Hotkey>) {
		const shortcut = hotkeyToShortcut(e.detail);
		if (shortcut !== undefined)
			shortcuts.addShortcut(shortcut);
		else
			console.log("Ignoring incomplete hotkey", e.detail);
	}

	function deleteHotkey(e: CustomEvent<Hotkey>) {
		const shortcut = hotkeyToShortcut(e.detail);
		if (shortcut !== undefined)
			shortcuts.deleteShortcut(shortcut);
		else
			console.log("Ignoring incomplete hotkey", e.detail);
	}

	function updateLoudness() {
		audioSett.loudnessThreshold = loudnessThreshold === minLoudnessThreshold ? null : loudnessThreshold;
		syncSettings();
	}

	function updateGlobalVolume() {
		audioSett.globalVolume = dbToFactor(globalVolume);
		syncSettings();
		// Update global volume instantly
		app.transientSettings.flush();
	}

	function browserNotificationChanged() {
		syncSettings();
		if (app.transientSettings.app.allowBrowserNotifications && Notification.permission === "default") {
			Notification.requestPermission();
		}
	}

	$: {
		// Subscribe to loadness changes when on audio tab
		if ($tablistIndex === 1) {
			loudnessSocket = new WebSocket(`${backend.wsBaseAddress}/loudness`);
			loudnessSocket.binaryType = "arraybuffer";
			loudnessSocket.onmessage = (ev) => {
				loudness = new DataView(ev.data).getFloat64(0);
			};
			loudnessSocket.onclose = () => {
				loudnessSocket = undefined;
			};
		} else {
			loudnessSocket?.close();
			loudnessSocket = undefined;
		}
	}

	onMount(() => {
		browserNotificationDenied = Notification.permission === "denied";
		return () => {
			loudnessSocket?.close();
			loudnessSocket = undefined;
		};
	})
</script>

<div class="settings">
	<BTabList bind:activeIndex={tablistIndex}>
		<BTabSlot title="App">
			<BKeyValue label="Ask before closing">
				<input
					type="checkbox"
					class="checkbox-switch is-info"
					bind:checked={app.transientSettings.app.askBeforeClosing}
					on:change={() => syncSettings()} />
			</BKeyValue>
			<BKeyValue label="Developer Mode">
				<input
					type="checkbox"
					class="checkbox-switch is-info"
					bind:checked={$developMode}
					on:change={() => syncSettings()} />
			</BKeyValue>
			<BKeyValue
				label="Browser notifications"
				title={browserNotificationDenied ? "Your browser blocked notifications for this page. If you want to use them, enable notifications in your browser settings and reload the page." : ""}>
				<input
					type="checkbox"
					class="checkbox-switch is-info"
					disabled={browserNotificationDenied}
					bind:checked={app.transientSettings.app.allowBrowserNotifications}
					on:change={browserNotificationChanged}>
			</BKeyValue>
		</BTabSlot>

		<BTabSlot title="Audio">
			<BKeyValue label="Global Volume">
				<div class="volumeControl">
					<BSlider
						min={minGlobalVolume}
						max={maxGlobalVolume}
						step={1}
						bind:value={globalVolume}
						display={(n) => `${n} dB`}
						tooltip={true}
						on:input={updateGlobalVolume} />
				</div>
			</BKeyValue>
			<div>Loudness:</div>
			<SimpleDiagram bind:this={loudnessDiagram}
				width={LOUDNESS_WIDTH}
				height={LOUDNESS_HEIGHT}
				min={LOUDNESS_MIN}
				max={LOUDNESS_MAX}
				count={LOUDNESS_COUNT}
				lines={[[-14, "#555555"], [loudnessThreshold, "#aa3333"]]}
			/>
			<BKeyValue label="Volume Capture Trigger">
				<div class="volumeControl">
					<BSlider
						min={minLoudnessThreshold}
						max={maxLoudnessThreshold}
						step={1}
						bind:value={loudnessThreshold}
						display={(n) => `${n} LUFS`}
						tooltip={true}
						on:input={updateLoudness} />
				</div>
			</BKeyValue>
		</BTabSlot>

		<BTabSlot title="TTS">
			<BKeyValue label="Voice" labelStyle="is-normal">
				<BDropDown
					items={voices}
					display={(v) => v.name}
					bind:selected={synthSett.voice}
					on:change={() => syncSettings()} />
			</BKeyValue>
			<BKeyValue label="Speed" labelStyle="is-normal">
				<BSlider
					min={0.1}
					max={3}
					step={0.1}
					bind:value={synthSett.speed}
					tooltip={true}
					on:change={() => syncSettings()} />
			</BKeyValue>
			<BKeyValue label="Volume" labelStyle="is-normal">
				<BSlider
					min={0}
					max={1}
					step={0.05}
					bind:value={synthSett.volume}
					tooltip={true}
					on:change={() => syncSettings()} />
			</BKeyValue>
			<BKeyValue label="Preview" narrow={false} labelStyle="is-normal">
				<div class="is-horizontal field">
					<div class="control" style="flex: 1;">
						<input
							bind:this={previewText}
							class="input"
							value="Mit Qwint wird alles besser" />
					</div>
					<div class="control">
						<button class="button" on:click={() => previewVoice()}>Listen</button>
					</div>
				</div>
			</BKeyValue>
		</BTabSlot>
		<BTabSlot title="Hotkeys">
			{#each hotkeyActions as hotkeyAction}
				<BHotkeyField hotkey={shortcutToHotkey(hotkeyAction)} on:change={changeHotkey} on:button={deleteHotkey} iconName="close" />
			{/each}
			{#if incompleteHotkey !== undefined}
				<BHotkeyField hotkey={incompleteHotkey} on:change={changeHotkey} on:button={deleteHotkey} iconName="close" />
			{/if}

			<BKeyValue label="Add shortcut" labelStyle="is-normal">
				<button class="button" on:click={createHotkey}>
					<Icon name="plus" />
				</button>
			</BKeyValue>
		</BTabSlot>
	</BTabList>
</div>

<style lang="scss">
	.volumeControl {
		display: flex;
		align-items: center;
	}

	.settings {
		padding: 1em;
	}
</style>
