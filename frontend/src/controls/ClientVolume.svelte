<script lang="typescript">
	import { Connection } from "../connection";
	import { Client } from "../tree/book";
	import BSlider from "../ui/BSlider.svelte";
	import Icon from "../ui/Icon.svelte";

	export let connection!: Connection;
	export let client!: Client;
	// Volume is in dB, https://www.dr-lex.be/info-stuff/volumecontrols.html
	let minVolume = -30;
	let maxVolume = +30;
	let clientVolume = client.volume;

	let volumeUpdated = false;
	let volumeTimer: number | undefined;

	async function loadVolume(hovered: boolean) {
		if (hovered) {
			volumeUpdated = false;
			await client.loadVolume();
			if (!volumeUpdated) {
				if ($clientVolume === 0) {
					$clientVolume = minVolume;
				} else {
					$clientVolume = Math.round(20 * Math.log10($clientVolume ?? 0));
				}
			}
		}
	}

	function toggleVolume() {
		if ($clientVolume === minVolume) {
			$clientVolume = 0;
		} else {
			$clientVolume = minVolume;
		}
	}

	$: if($clientVolume !== undefined) updateVolume();

	function updateVolume() {
		volumeUpdated = true;
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
</script>

<div class="volumeControl">
<button class="volume button" on:click={toggleVolume}>
	{#if $clientVolume === minVolume}
		<Icon name="volume-off" />
	{:else}
		<Icon name="volume-high" />
	{/if}
</button>
<BSlider min={minVolume} max={maxVolume} step={1} bind:value={$clientVolume} display={n => `${n} dB`} tooltip={true} />
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
