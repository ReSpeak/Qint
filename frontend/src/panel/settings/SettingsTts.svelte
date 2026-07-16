<script lang="ts">
	import { app } from "../../app";
	import TabSlot from "../../ui/container/TabSlot.svelte";
	import KeyValue from "../../ui/util/KeyValue.svelte";
	import DropDown from "../../ui/html/DropDown.svelte";
	import Slider from "../../ui/html/Slider.svelte";

	const synthSett = app.settings.synth;
	const voices = synthSett._voices;
	let previewText!: HTMLInputElement;

	function syncSettings() {
		app.settings.save();
	}

	function previewVoice() {
		const text = previewText.value;
		synthSett.trySpeak(text);
	}
</script>

<TabSlot title="Text to Speech">
	<KeyValue label="Voice" labelStyle="is-normal">
		<DropDown
			items={$voices}
			display={(v) => v.name}
			compare={(a, b) => a.voiceURI === b?.voiceURI && a.name === b?.name}
			bind:selected={synthSett.voice}
			on:change={() => syncSettings()}
		/>
	</KeyValue>
	<KeyValue label="Speed" labelStyle="is-normal">
		<Slider
			min={0.1}
			max={3}
			step={0.1}
			bind:value={synthSett.speed}
			tooltip={true}
			on:change={() => syncSettings()}
		/>
	</KeyValue>
	<KeyValue label="Volume" labelStyle="is-normal">
		<Slider
			min={0}
			max={1}
			step={0.05}
			bind:value={synthSett.volume}
			tooltip={true}
			on:change={() => syncSettings()}
		/>
	</KeyValue>
	<KeyValue label="Preview" narrow={false} labelStyle="is-normal">
		<div class="is-horizontal field">
			<div class="control" style="flex: 1;">
				<input bind:this={previewText} class="input" value="Mit Qwint wird alles besser" />
			</div>
			<div class="control">
				<button class="button" on:click={() => previewVoice()}>Listen</button>
			</div>
		</div>
	</KeyValue>
</TabSlot>
