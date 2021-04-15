<script lang="ts">
	import { onDestroy } from "svelte";
	import { app } from "../../app";
	import { backend } from "../../backend/backend";
	import {
		dbToFactor,
		factorToDb,
		LOUDNESS_END_MAGIC,
		LOUDNESS_MAX,
		LOUDNESS_MIN,
		MIN_VOLUME_DB,
		NARROW_NO_BREAK_SPACE,
		on,
	} from "../../util";
	import BTabSlot from "../../ui/BTabSlot.svelte";
	import BKeyValue from "../../ui/BKeyValue.svelte";
	import BSlider from "../../ui/BSlider.svelte";
	import SimpleDiagram from "../../ui/UiSimpleDiagram.svelte";

	let selected: boolean;
	const audioSett = app.transientSettings.audio;

	let minGlobalVolume = MIN_VOLUME_DB;
	let maxGlobalVolume = -MIN_VOLUME_DB;
	let minLoudnessThreshold = -100;
	let maxLoudnessThreshold = 100;

	let globalVolume = factorToDb(audioSett.globalVolume);
	let loudnessThreshold = audioSett.loudnessThreshold ?? minLoudnessThreshold;
	let loudnessDiagram: SimpleDiagram;
	let renderRequested: boolean = false;

	let loudnessSocket: WebSocket | undefined;
	let loudness: number | undefined;
	const LOUDNESS_WIDTH = 1000;
	const LOUDNESS_HEIGHT = 300;
	const LOUDNESS_COUNT = 500;

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

	function requestRenderLoudnessGraphs() {
		if (renderRequested) return;
		renderRequested = true;
		requestAnimationFrame((ts) => renderLoudnessGraphs(ts));
	}

	function renderLoudnessGraphs(timestamp: number) {
		renderRequested = false;
		let hasRequest = loudnessDiagram?.redraw(timestamp) ?? false;
		if (hasRequest) {
			requestRenderLoudnessGraphs();
		}
	}

	$: on(selected, changeSelected());

	function changeSelected() {
		if (selected && loudnessSocket === undefined) {
			console.log("mount audio");
			loudnessSocket = new WebSocket(`${backend.wsBaseAddress}/loudness`);
			loudnessSocket.binaryType = "arraybuffer";
			loudnessSocket.onmessage = (ev) => {
				const now = performance.now();
				loudness = new DataView(ev.data).getFloat64(0);
				if (loudness !== LOUDNESS_END_MAGIC) {
					loudnessDiagram?.addValue(loudness, now);
					requestRenderLoudnessGraphs();
				}
			};
			loudnessSocket.onclose = () => {
				loudnessSocket = undefined;
			};
		} else {
			closeSocket();
		}
	}

	function closeSocket() {
		console.log("unmount audio");
		loudnessSocket?.close();
		loudnessSocket = undefined;
	}

	onDestroy(() => {
		closeSocket();
	});
</script>

<BTabSlot title="Audio" bind:selected>
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
	<BKeyValue label="Loudness">
		<SimpleDiagram
			bind:this={loudnessDiagram}
			width={LOUDNESS_WIDTH}
			height={LOUDNESS_HEIGHT}
			min={LOUDNESS_MIN}
			max={LOUDNESS_MAX}
			count={LOUDNESS_COUNT}
			lines={[
				[-14, `Standard normalized volume (-14${NARROW_NO_BREAK_SPACE}dB)`, "#555555"],
				[loudnessThreshold, "Your talking threshold", "#aa3333"],
			]} />
	</BKeyValue>
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
