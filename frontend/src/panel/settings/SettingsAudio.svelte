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
	import TabSlot from "../../ui/container/TabSlot.svelte";
	import KeyValue from "../../ui/util/KeyValue.svelte";
	import Slider from "../../ui/html/Slider.svelte";
	import DropDown from "../../ui/html/DropDown.svelte";
	import VoiceGraph from "../../ui/specialized/VoiceGraph.svelte";

	let selected: boolean;
	const audioSett = app.transientSettings.audio;

	const minGlobalVolume = MIN_VOLUME_DB;
	const maxGlobalVolume = -MIN_VOLUME_DB;
	const minLoudnessThreshold = -100;
	const maxLoudnessThreshold = 100;

	let globalVolume = factorToDb(audioSett.globalVolume);
	let loudnessThreshold = audioSett.loudnessThreshold ?? minLoudnessThreshold;
	let loudnessDiagram: VoiceGraph;
	let renderRequested: boolean = false;

	let loudnessSocket: WebSocket | undefined;
	let loudness: number | undefined;
	const LOUDNESS_WIDTH = 1000;
	const LOUDNESS_HEIGHT = 300;
	const LOUDNESS_COUNT = 500;

	type DeviceList = [null, ...string[]];
	let captureDevices: DeviceList = [null];
	let playbackDevices: DeviceList = [null];
	let selectedCaptureDevice: string | null = audioSett.capture;
	let selectedPlaybackDevice: string | null = audioSett.playback;

	function syncSettings() {
		app.transientSettings.save();
	}
	function syncSettingsImmediately() {
		app.transientSettings.save();
		app.transientSettings.flush();
	}

	function updateLoudness() {
		audioSett.loudnessThreshold =
			loudnessThreshold === minLoudnessThreshold ? undefined : loudnessThreshold;
		syncSettings();
	}

	function updateGlobalVolume() {
		audioSett.globalVolume = dbToFactor(globalVolume);
		syncSettingsImmediately();
	}

	function requestRenderLoudnessGraphs() {
		if (renderRequested) return;
		renderRequested = true;
		requestAnimationFrame((ts) => renderLoudnessGraphs(ts));
	}

	function renderLoudnessGraphs(timestamp: number) {
		renderRequested = false;
		const hasRequest = loudnessDiagram?.redraw(timestamp) ?? false;
		if (hasRequest) {
			requestRenderLoudnessGraphs();
		}
	}

	async function fetchAvailableDevices() {
		const list = await backend.get_audio_device_list();
		captureDevices = [null, ...list.capture];
		playbackDevices = [null, ...list.playback];
	}

	function changeAudioDevice() {
		audioSett.capture = selectedCaptureDevice;
		audioSett.playback = selectedPlaybackDevice;
		syncSettingsImmediately();
	}

	$: on(selected, changeSelected());

	function changeSelected() {
		if (selected && loudnessSocket === undefined) {
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

		fetchAvailableDevices();
	}

	function closeSocket() {
		loudnessSocket?.close();
		loudnessSocket = undefined;
	}

	fetchAvailableDevices();

	onDestroy(() => {
		closeSocket();
	});
</script>

<TabSlot title="Audio" bind:selected>
	<KeyValue label="Capture Device">
		<DropDown
			items={captureDevices}
			display={(d) => (d === null ? "System Default" : d)}
			bind:selected={selectedCaptureDevice}
			on:change={changeAudioDevice} />
	</KeyValue>
	<KeyValue label="Playback Device">
		<DropDown
			items={playbackDevices}
			display={(d) => (d === null ? "System Default" : d)}
			bind:selected={selectedPlaybackDevice}
			on:change={changeAudioDevice} />
	</KeyValue>

	<KeyValue label="Global Volume">
		<div class="volumeControl">
			<Slider
				min={minGlobalVolume}
				max={maxGlobalVolume}
				step={1}
				bind:value={globalVolume}
				display={(n) => `${n} dB`}
				tooltip={true}
				on:input={updateGlobalVolume} />
		</div>
	</KeyValue>
	<KeyValue label="Loudness">
		<VoiceGraph
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
	</KeyValue>
	<KeyValue label="Volume Capture Trigger">
		<div class="volumeControl">
			<Slider
				min={minLoudnessThreshold}
				max={maxLoudnessThreshold}
				step={1}
				bind:value={loudnessThreshold}
				display={(n) => `${n} LUFS`}
				tooltip={true}
				on:input={updateLoudness} />
		</div>
	</KeyValue>
</TabSlot>

<style lang="scss">
	.volumeControl {
		display: flex;
		align-items: center;
	}
</style>
