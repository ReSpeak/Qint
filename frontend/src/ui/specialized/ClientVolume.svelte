<script lang="ts">
	import { onMount } from "svelte";
	import { Connection } from "../../connection";
	import { Client } from "../../book";
	import Slider from "../html/Slider.svelte";
	import Icon from "../icon/Icon.svelte";
	import { dbToFactor, debounced, MIN_VOLUME_DB } from "../../util";

	export let connection: Connection;
	export let client: Client;
	const minVolume = MIN_VOLUME_DB;
	const maxVolume = -MIN_VOLUME_DB;
	const clientVolume = client.volume;

	async function loadVolume() {
		await client.loadVolume();
	}

	function toggleVolume() {
		if ($clientVolume === minVolume) {
			$clientVolume = 0;
		} else {
			$clientVolume = minVolume;
		}
		updateVolume();
	}

	const updateVolume = debounced(() => {
		client.updateVolume(connection, dbToFactor($clientVolume));
	}, 100);

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
	<Slider
		min={minVolume}
		max={maxVolume}
		step={1}
		bind:value={$clientVolume}
		display={(n) => `${n} dB`}
		tooltip={true}
		on:input={updateVolume} />
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
