<script lang="typescript">
	import { onDestroy } from "svelte";
	import { get, writable } from "svelte/store";
	import { Connection } from "../connection";
	import Icon from "../ui/Icon.svelte";
	import BTabList from "../ui/BTabList.svelte";
	import BTabSlot from "../ui/BTabSlot.svelte";

	export let connection!: Connection;
	let loudness = connection.loudness;

	let minLoudnessThreshold = -100;
	let maxLoudnessThreshold = 0;
	let loudnessThreshold = minLoudnessThreshold;
	let loudnessUpdated = false;
	let loudnessTimer: number | undefined;

	connection.sendMessage({ SubscribeLoudness: true });

	function updateLoudness() {
		loudnessUpdated = true;
		if (loudnessTimer) return;
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
	<BTabList>
		<BTabSlot title="Audio">
			<div>Loudness: {$loudness}</div>
			<input
				type="range"
				min="{minLoudnessThreshold}"
				max="{maxLoudnessThreshold}"
				step="2"
				bind:value="{loudnessThreshold}"
				class="volume slider"
				on:input="{updateLoudness}"
			/>
		</BTabSlot>

		<BTabSlot title="TTS">
			<div class="field is-horizontal">
				<div class="field-label is-normal">
					<label class="label">Department</label>
				</div>
				<div class="field-body">
					<div class="field is-narrow">
						<div class="control">
							<div class="select is-fullwidth">
								<select>
									<option>Business development</option>
									<option>Marketing</option>
									<option>Sales</option>
								</select>
							</div>
						</div>
					</div>
				</div>
			</div>
		</BTabSlot>
	</BTabList>
</div>

<style lang="scss">
	.settings {
		padding: 1em;
	}
</style>
