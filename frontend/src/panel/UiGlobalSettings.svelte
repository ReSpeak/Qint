<script lang="typescript">
	import { onDestroy, onMount } from "svelte";
	import type { Writable } from "svelte/store";
	import { Connection } from "../connection";
	import { app } from "../app";
	import BTabList from "../ui/BTabList.svelte";
	import BTabSlot from "../ui/BTabSlot.svelte";
	import BDropDown from "../ui/BDropDown.svelte";
	import BKeyValue from "../ui/BKeyValue.svelte";
	import BSlider from "../ui/BSlider.svelte";
	import type { SettGroup } from "../transientSettings";
	import { dbToFactor, factorToDb, MIN_VOLUME_DB } from "../util";

	export let connection: Connection;

	let tablistIndex: Writable<number>;
	let loudness = connection.loudness;
	const audioSett = app.transientSettings.audio;

	let developMode = app.transientSettings.ui._developMode;
	let minGlobalVolume = MIN_VOLUME_DB;
	let maxGlobalVolume = -MIN_VOLUME_DB;
	let minLoudnessThreshold = -100;
	let maxLoudnessThreshold = 100;
	let browserNotificationDenied = false;

	let globalVolume = factorToDb(audioSett.globalVolume);
	let loudnessThreshold = audioSett.loudnessThreshold ?? minLoudnessThreshold;

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

	function syncSettings(group?: SettGroup) {
		app.transientSettings.save(group);
	}

	function updateLoudness() {
		audioSett.loudnessThreshold = loudnessThreshold === minLoudnessThreshold ? null : loudnessThreshold;
		syncSettings('audio');
	}

	function updateGlobalVolume() {
		audioSett.globalVolume = dbToFactor(globalVolume);
		syncSettings('audio');
		// Update global volume instantly
		app.transientSettings.flush();
	}

	function browserNotificationChanged() {
		syncSettings('app');
		if (app.transientSettings.app.allowBrowserNotifications && Notification.permission === "default") {
			Notification.requestPermission();
		}
	}

	$: {
		// Subscribe to loadness changes when on audio tab
		connection.sendMessage({ SubscribeLoudness: $tablistIndex === 1 });
	}

	onMount(() => {
		browserNotificationDenied = Notification.permission === "denied";
	})

	onDestroy(() => {
		connection.sendMessage({ SubscribeLoudness: false });
	});
</script>

<div class="settings">
	<BTabList bind:activeIndex={tablistIndex}>
		<BTabSlot title="App">
			<BKeyValue label="Ask before closing">
				<input
					type="checkbox"
					class="checkbox-switch is-info"
					bind:checked={app.transientSettings.app.askBeforeClosing}
					on:change={() => syncSettings('app')} />
			</BKeyValue>
			<BKeyValue label="Developer Mode">
				<input
					type="checkbox"
					class="checkbox-switch is-info"
					bind:checked={$developMode}
					on:change={() => syncSettings('ui')} />
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
			<div>Loudness: {$loudness}</div>
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
					on:change={() => syncSettings('synth')} />
			</BKeyValue>
			<BKeyValue label="Speed" labelStyle="is-normal">
				<BSlider
					min={0.1}
					max={3}
					step={0.1}
					bind:value={synthSett.speed}
					tooltip={true}
					on:change={() => syncSettings('synth')} />
			</BKeyValue>
			<BKeyValue label="Volume" labelStyle="is-normal">
				<BSlider
					min={0}
					max={1}
					step={0.05}
					bind:value={synthSett.volume}
					tooltip={true}
					on:change={() => syncSettings('synth')} />
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
