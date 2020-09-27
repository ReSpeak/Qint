<script lang="typescript">
	import { onMount } from "svelte";
	import { Connection } from "../connection";
	import { Client } from "../book";
	import BSlider from "./BSlider.svelte";
	import Icon from "./Icon.svelte";

	export let connection: Connection;
	export let client: Client;
	// Volume is in dB, https://www.dr-lex.be/info-stuff/volumecontrols.html
	let minVolume = -30;
	let maxVolume = +30;
	let clientVolume = client.volume;

	let volumeTimer: number | undefined;

	async function loadVolume() {
		await client.loadVolume();
	}

	function toggleVolume() {
		if ($clientVolume === minVolume) {
			$clientVolume = 0;
		} else {
			$clientVolume = minVolume;
		}
	}

	function updateVolume() {
		if (volumeTimer)
			return;
		// Update every few ms
		volumeTimer = setTimeout(() => {
			volumeTimer = undefined;
			let vol = 0;
			if ($clientVolume !== minVolume) {
				vol = Math.pow(10, $clientVolume / 20);
			}
			client.updateVolume(connection, vol);
		}, 100);
	}

	onMount(() => {
		loadVolume();
	});
</script>

<div class="volumeControl">
<button class="volume button" on:click={toggleVolume}>
	{#if $clientVolume === minVolume}
		<Icon name="volume-off" />
	{:else}
		<Icon name="volume-high" />
	{/if}
</button>
<BSlider min={minVolume} max={maxVolume} step={1} bind:value={$clientVolume} display={n => `${n} dB`} tooltip={true} on:input={updateVolume} />
</div>

<style lang="scss">
	.volumeControl {
		display: flex;
		align-items: center;
	}

	.volume.button {
		margin-right: 0.5em;
	}
</style>
