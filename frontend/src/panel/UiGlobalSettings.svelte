<script lang="typescript">
	import { onDestroy } from "svelte";
	import { Connection } from "../connection";
	import { app } from "../app";
	import BTabList from "../ui/BTabList.svelte";
	import BTabSlot from "../ui/BTabSlot.svelte";
	import BDropDown from "../ui/BDropDown.svelte";
	import BKeyValue from "../ui/BKeyValue.svelte";
	import BSlider from "../ui/BSlider.svelte";
	import type { SettGroup } from "../transientSettings";

	export let connection: Connection;
	let loudness = connection.loudness;

	let minLoudnessThreshold = -100;
	let maxLoudnessThreshold = 0;
	let loudnessThreshold = minLoudnessThreshold;
	let loudnessTimer: number | undefined;

	connection.sendMessage({ SubscribeLoudness: true });

	function updateLoudness() {
		if (loudnessTimer !== undefined) return;
		// Update every few ms
		loudnessTimer = setTimeout(() => {
			loudnessTimer = undefined;
			connection.sendMessage({ SetLoudnessThreshold: loudnessThreshold });
		}, 100);
	}

	function syncSettings(group?: SettGroup) {
		app.transientSettings.save(group);
	}

	// Reload settings
	app.transientSettings.loadAsync();

	// Text-to-Speech
	const synthSett = app.transientSettings.synth;
	const voices = synthSett.getVoices();
	let previewText!: HTMLInputElement;
	function previewVoice() {
		const text = previewText.value;
		synthSett.trySpeak(text);
	}

	onDestroy(() => {
		connection.sendMessage({ SubscribeLoudness: false });
	});
</script>

<div class="settings">
	<BTabList>
		<BTabSlot title="App">
			<BKeyValue label="Ask before closing">
				<input
					type="checkbox"
					bind:checked={app.transientSettings.app.askBeforeClosing}
					on:change={() => syncSettings('app')} />
			</BKeyValue>
		</BTabSlot>

		<BTabSlot title="Audio">
			<BKeyValue label="Volume trigger">
				<div>Loudness: {$loudness}</div>
				<input
					type="range"
					min={minLoudnessThreshold}
					max={maxLoudnessThreshold}
					step="2"
					bind:value={loudnessThreshold}
					class="volume slider"
					on:input={updateLoudness} />
			</BKeyValue>
		</BTabSlot>

		<BTabSlot title="TTS">
			<BKeyValue label="Voice">
				<BDropDown
					items={voices}
					display={(v) => v.name}
					bind:selected={synthSett.voice}
					on:change={() => syncSettings('synth')} />
			</BKeyValue>
			<BKeyValue label="Speed">
				<BSlider
					min={0.1}
					max={3}
					step={0.1}
					bind:value={synthSett.speed}
					tooltip={true}
					on:change={() => syncSettings('synth')} />
			</BKeyValue>
			<BKeyValue label="Volume">
				<BSlider
					min={0}
					max={1}
					step={0.05}
					bind:value={synthSett.volume}
					tooltip={true}
					on:change={() => syncSettings('synth')} />
			</BKeyValue>
			<BKeyValue label="Preview" narrow={false}>
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

<style>
	.settings {
		padding: 1em;
	}
</style>
