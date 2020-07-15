<script lang="typescript">
	import { onDestroy } from "svelte";
	import { get, writable } from "svelte/store";
	import { Connection } from "../connection";
	import Icon from "../ui/Icon.svelte";
	import BTabList from "../ui/BTabList.svelte";
	import BTabSlot from "../ui/BTabSlot.svelte";
	import BDropDown from "../ui/BDropDown.svelte";
	import BKeyValue from "../ui/BKeyValue.svelte";
	import BSlider from "../ui/BSlider.svelte";

	export let connection!: Connection;
	let loudness = connection.loudness;

	let minLoudnessThreshold = -100;
	let maxLoudnessThreshold = 0;
	let loudnessThreshold = minLoudnessThreshold;
	let loudnessUpdated = false;
	let loudnessTimer: number | undefined;

	connection.sendMessage({ SubscribeLoudness: true });

	function updateLoudness() {
		loudnessUpdated = true;
		if (loudnessTimer) return;
		// Update every few ms
		loudnessTimer = setTimeout(() => {
			loudnessTimer = undefined;
			connection.sendMessage({ SetLoudnessThreshold: loudnessThreshold });
		}, 100);
	}

	// Text-to-Speech
	const synth = window.speechSynthesis;
	const synthSett = connection.volatileSettings.synth;
	console.log(synthSett);
	let voices = synth.getVoices();
	let previewText!: HTMLInputElement;
	function previewVoice() {
		const text = previewText.value;
		const utter = synthSett.getNewUtter();
		utter.text = text;
		synth.cancel();
		synth.speak(utter);
	}

	onDestroy(() => {
		connection.sendMessage({ SubscribeLoudness: false });
	});
</script>

<div class="settings">
	<BTabList>
		<BTabSlot title="Audio">
			<BKeyValue label="Volume trigger">
				<div>Loudness: {$loudness}</div>
				<input
					type="range"
					min="{minLoudnessThreshold}"
					max="{maxLoudnessThreshold}"
					step="2"
					bind:value="{loudnessThreshold}"
					class="volume slider"
					on:input="{updateLoudness}"
				/>
			</BKeyValue>
		</BTabSlot>

		<BTabSlot title="TTS">
			<BKeyValue label="Voice">
				<BDropDown
					items="{voices}"
					display="{v => v.name}"
					bind:selected="{synthSett.voice}"
				/>
			</BKeyValue>
			<BKeyValue label="Speed">
				<BSlider
					min="{0.1}"
					max="{3}"
					step="{0.1}"
					bind:value="{synthSett.speed}"
					tooltip="{true}"
				/>
			</BKeyValue>
			<BKeyValue label="Volume">
				<BSlider
					min="{0}"
					max="{1}"
					step="{0.05}"
					bind:value="{synthSett.volume}"
					tooltip="{true}"
				/>
			</BKeyValue>
			<BKeyValue label="Preview" narrow="{false}">
				<div class="is-horizontal field">
					<div class="control" style="flex: 1;">
						<input
							bind:this="{previewText}"
							class="input"
							value="Mit Qwint wird alles besser"
						/>
					</div>
					<div class="control">
						<button class="button" on:click="{() => previewVoice()}">Listen</button>
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
