<script lang="typescript">
	import { onMount } from "svelte";
	import { app } from "../../app";
	import { backend } from "../../backend/backend";
	import {
		dbToFactor,
		factorToDb,
		LOUDNESS_END_MAGIC,
		LOUDNESS_MAX,
		LOUDNESS_MIN,
		LOUDNESS_UPDATE_MS,
		MIN_VOLUME_DB,
	} from "../../util";
	import BTabSlot from "../../ui/BTabSlot.svelte";
	import BKeyValue from "../../ui/BKeyValue.svelte";
	import BSlider from "../../ui/BSlider.svelte";
	import SimpleDiagram from "../../ui/UiSimpleDiagram.svelte";

	const audioSett = app.transientSettings.audio;

	let minGlobalVolume = MIN_VOLUME_DB;
	let maxGlobalVolume = -MIN_VOLUME_DB;
	let minLoudnessThreshold = -100;
	let maxLoudnessThreshold = 100;

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

	function syncSettings() {
		app.transientSettings.save();
	}
	
	function updateLoudness() {
		audioSett.loudnessThreshold =
			loudnessThreshold === minLoudnessThreshold ? null : loudnessThreshold;
		syncSettings();
	}

	function updateGlobalVolume() {
		audioSett.globalVolume = dbToFactor(globalVolume);
		syncSettings();
		// Update global volume instantly
		app.transientSettings.flush();
	}

	onMount(() => {
		// TODO check how this behaves in our tablist
		// Subscribe to loadness changes when this is mounted
		loudnessSocket = new WebSocket(`${backend.wsBaseAddress}/loudness`);
		loudnessSocket.binaryType = "arraybuffer";
		loudnessSocket.onmessage = (ev) => {
			loudness = new DataView(ev.data).getFloat64(0);
		};
		loudnessSocket.onclose = () => {
			loudnessSocket = undefined;
		};

		return () => {
			loudnessSocket?.close();
			loudnessSocket = undefined;
		};
	});
</script>

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
	<SimpleDiagram
		bind:this={loudnessDiagram}
		width={LOUDNESS_WIDTH}
		height={LOUDNESS_HEIGHT}
		min={LOUDNESS_MIN}
		max={LOUDNESS_MAX}
		count={LOUDNESS_COUNT}
		lines={[
			[-14, "#555555"],
			[loudnessThreshold, "#aa3333"],
		]} />
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

<style lang="scss">
	.volumeControl {
		display: flex;
		align-items: center;
	}
</style>
