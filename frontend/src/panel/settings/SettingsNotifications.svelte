<script lang="ts">
	import { app } from "../../app";
	import TabSlot from "../../ui/container/TabSlot.svelte";

	const settings = app.transientSettings.notifications;

	const notificationCategories = [
		{ name: "Poke/Mention", attr: settings.poke },
		{ name: "Message", attr: settings.message },
		{ name: "Server/Channel changed", attr: settings.channelChanged },
		{ name: "Client changed", attr: settings.clientChanged },
		{ name: "Client switched channel", attr: settings.clientSwitched },
		{
			name: "Client changed state",
			attr: settings.clientStateChanged,
			description: "E.g. mute or unmute",
		},
	];

	function syncSettings() {
		app.transientSettings.save();
	}
</script>

<TabSlot title="Notifications">
	<table class="table">
		<thead>
			<tr
				><td /><td>Text to speech</td><td>Notification</td><td
					><abbr title="Notify only if it affects your client or your current channel"
						>Only relevant notifcations</abbr
					></td
				></tr>
		</thead>
		<tbody>
			{#each notificationCategories as cat}
				<tr>
					<td>
						{#if "description" in cat}
							<abbr title={cat.description}>{cat.name}</abbr>
						{:else}
							{cat.name}
						{/if}
					</td>
					<td>
						<div class="checkboxCell">
							<input
								type="checkbox"
								class="checkbox-switch is-info"
								bind:checked={cat.attr.tts}
								on:change={syncSettings} />
						</div>
					</td>
					<td>
						<div class="checkboxCell">
							<input
								type="checkbox"
								class="checkbox-switch is-info"
								bind:checked={cat.attr.notification}
								on:change={syncSettings} />
						</div>
					</td>
					<td>
						{#if "onlyRelevant" in cat.attr}
							<div class="checkboxCell">
								<input
									type="checkbox"
									class="checkbox-switch is-info"
									bind:checked={cat.attr.onlyRelevant}
									on:change={syncSettings} />
							</div>
						{/if}
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</TabSlot>

<style lang="scss">
	.checkboxCell {
		display: flex;
		justify-content: center;
	}
</style>
