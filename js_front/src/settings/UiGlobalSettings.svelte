<script lang="typescript">
	import { onDestroy } from "svelte";
	import { get, writable } from "svelte/store";
	import { Connection } from "../connection";
	import Icon from "../ui/Icon.svelte";

	export let connection!: Connection;
	let loudness = connection.loudness;

	let minLoudnessThreshold = -100;
	let maxLoudnessThreshold = 0;
	let loudnessThreshold = minLoudnessThreshold;
	let loudnessUpdated = false;;
	let loudnessTimer: number | undefined;

	connection.sendMessage({ SubscribeLoudness: true });

	function updateLoudness() {
		loudnessUpdated = true;
		if (loudnessTimer)
			return;
		// Update every few ms
		loudnessTimer = setTimeout(() => {
			loudnessTimer = undefined;
			connection.sendMessage({ SetLoudnessThreshold: loudnessThreshold });
		}, 100);
	}

	onDestroy(() => {
		connection.sendMessage({ SubscribeLoudness: false });
	});
</script>

<div class="settings">
	<h1>Settings</h1>
	<div>Loudness: {$loudness}</div>
	<input type="range" min={minLoudnessThreshold} max={maxLoudnessThreshold} step="2" bind:value={loudnessThreshold}
		class="volume slider" on:input={updateLoudness} />
</div>

<style lang="scss">
</style>
