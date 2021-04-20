<script lang="ts">
	import { app } from "../../app";
	import BTabSlot from "../../ui/BTabSlot.svelte";
	import BKeyValue from "../../ui/BKeyValue.svelte";
	import BDropDown from "../../ui/BDropDown.svelte";
	import BSlider from "../../ui/BSlider.svelte";

	const synthSett = app.transientSettings.synth;
	const voices = synthSett.getVoices();
	let previewText!: HTMLInputElement;

	function syncSettings() {
		app.transientSettings.save();
	}

	function previewVoice() {
		const text = previewText.value;
		synthSett.trySpeak(text);
	}
</script>

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
				<input bind:this={previewText} class="input" value="Mit Qwint wird alles besser" />
			</div>
			<div class="control">
				<button class="button" on:click={() => previewVoice()}>Listen</button>
			</div>
		</div>
	</BKeyValue>
</BTabSlot>
