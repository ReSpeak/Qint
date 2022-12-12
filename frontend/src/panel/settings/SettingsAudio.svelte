<script lang="ts">
	import { onDestroy } from "svelte";
	import { app } from "../../app";
	import { backend } from "../../backend/backend";
	import type { LoudnessUnsubscribe } from "../../backend/backend";
	import {
		dbToFactor,
		factorToDb,
		LOUDNESS_END_MAGIC,
		LOUDNESS_MAX,
		LOUDNESS_MIN_SETTINGS,
		MIN_VOLUME_DB,
		NARROW_NO_BREAK_SPACE,
		on,
		VAD_MAX,
		VAD_MIN,
	} from "../../util";
	import TabSlot from "../../ui/container/TabSlot.svelte";
	import KeyValue from "../../ui/util/KeyValue.svelte";
	import Slider from "../../ui/html/Slider.svelte";
	import DropDown from "../../ui/html/DropDown.svelte";
	import VoiceGraph from "../../ui/specialized/VoiceGraph.svelte";

	let selected: boolean;
	const audioSett = app.settings.audio;
	const developMode = app.settings.ui._developMode;

	const minGlobalVolume = MIN_VOLUME_DB;
	const maxGlobalVolume = -MIN_VOLUME_DB;
	const minVadThreshold = 0;
	const maxVadThreshold = 1;
	const defaultVadThreshold = 0.3;
	const minLoudnessThreshold = -100;
	const maxLoudnessThreshold = 0;

	let globalVolume = factorToDb(audioSett.globalVolume);
	let loudnessThreshold = audioSett.loudnessThreshold ?? minLoudnessThreshold;
	let vadThreshold = audioSett.vadThreshold ?? defaultVadThreshold;
	let loudnessDiagram: VoiceGraph;
	let vadDiagram: VoiceGraph;
	let renderRequested: boolean = false;

	let loudnessUnsub: LoudnessUnsubscribe | undefined;
	const LOUDNESS_WIDTH = 1000;
	const LOUDNESS_HEIGHT = 200;
	const LOUDNESS_COUNT = 500;

	type DeviceList = [null, ...string[]];
	let captureDevices: DeviceList = [null];
	let playbackDevices: DeviceList = [null];
	let selectedCaptureDevice: string | null = audioSett.capture;
	let selectedPlaybackDevice: string | null = audioSett.playback;

	function syncSettingsImmediately() {
		app.settings.save();
		app.settings.flush();
	}

	function updateLoudness() {
		audioSett.loudnessThreshold =
			loudnessThreshold === minLoudnessThreshold ? undefined : loudnessThreshold;
		syncSettingsImmediately();
	}

	function updateVad() {
		audioSett.vadThreshold = vadThreshold;
		syncSettingsImmediately();
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
		const loudnessHasRequest = loudnessDiagram?.redraw(timestamp) ?? false;
		const vadHasRequest = vadDiagram?.redraw(timestamp) ?? false;
		if (loudnessHasRequest || vadHasRequest) {
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
		if (selected && loudnessUnsub === undefined) {
			loudnessUnsub = backend.get_loudness_listener(([loudness, vad]) => {
				if (loudness !== LOUDNESS_END_MAGIC) {
					const now = performance.now();
					loudnessDiagram?.addValue(loudness, now);
					vadDiagram?.addValue(vad, now);
					requestRenderLoudnessGraphs();
				}
			});
		} else {
			closeSocket();
		}

		fetchAvailableDevices();
	}

	function closeSocket() {
		loudnessUnsub?.();
		loudnessUnsub = undefined;
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
			on:change={changeAudioDevice}
		/>
	</KeyValue>
	<KeyValue label="Playback Device">
		<DropDown
			items={playbackDevices}
			display={(d) => (d === null ? "System Default" : d)}
			bind:selected={selectedPlaybackDevice}
			on:change={changeAudioDevice}
		/>
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
				on:input={updateGlobalVolume}
			/>
		</div>
	</KeyValue>
	<KeyValue label="Loudness">
		<div class="field is-horizontal">
			<div class="field rotatedSideControl">
				<div class="volumeControl">
					<Slider
						min={minLoudnessThreshold}
						max={maxLoudnessThreshold}
						step={1}
						bind:value={loudnessThreshold}
						display={(n) => `${n}${NARROW_NO_BREAK_SPACE}LUFS`}
						tooltip={true}
						on:input={updateLoudness}
					/>
				</div>
			</div>
			<div class="field" style="flex:1;">
				<VoiceGraph
					bind:this={loudnessDiagram}
					width={LOUDNESS_WIDTH}
					height={LOUDNESS_HEIGHT}
					min={LOUDNESS_MIN_SETTINGS}
					max={LOUDNESS_MAX}
					count={LOUDNESS_COUNT}
					lines={[
						[
							-14,
							`Standard normalized volume (-14${NARROW_NO_BREAK_SPACE}dB)`,
							"#555555",
						],
						[loudnessThreshold, "Your talking threshold", "#aa3333"],
					]}
				/>
			</div>
		</div>
	</KeyValue>
	{#if $developMode}
		<KeyValue label="Voice Activation Detection">
			<div class="field is-horizontal">
				<div class="field rotatedSideControl">
					<div class="volumeControl">
						<Slider
							min={minVadThreshold}
							max={maxVadThreshold}
							step={0.01}
							bind:value={vadThreshold}
							display={(n) => `${Math.floor(n * 100)}${NARROW_NO_BREAK_SPACE}%`}
							tooltip={true}
							on:input={updateVad}
						/>
					</div>
				</div>
				<div class="field" style="flex:1;">
					<VoiceGraph
						bind:this={vadDiagram}
						width={LOUDNESS_WIDTH}
						height={LOUDNESS_HEIGHT}
						min={VAD_MIN}
						max={VAD_MAX}
						count={LOUDNESS_COUNT}
						lines={[
							[0.3, "Suggested VAD", "#555555"],
							[vadThreshold, "Your vad threshold", "#aa3333"],
						]}
						gradient={[
							[0.0, "#025189"],
							[0.7, "#81C6EB"],
							[1, "#20FF20"],
						]}
					/>
				</div>
			</div>
		</KeyValue>
	{/if}
</TabSlot>

<style lang="scss">
	.volumeControl {
		display: flex;
		align-items: center;
	}

	.rotatedSideControl {
		position: relative;

		> .volumeControl {
			position: absolute;
			$heigth: 200px;
			$heigth2: math.div($heigth, 2);
			width: $heigth;
			transform: translate(-50%, -50%) translate(0, $heigth2) rotate(270deg);

			> :global(.bslider) {
				flex: 1;
				margin: 0;
			}
		}
	}

	.field > :global(canvas) {
		border: 1px solid gray;
	}
</style>
