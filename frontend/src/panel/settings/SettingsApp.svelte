<script lang="ts">
	import { app } from "../../app";
	import TabSlot from "../../ui/container/TabSlot.svelte";
	import KeyValue from "../../ui/util/KeyValue.svelte";

	let browserNotificationPermission = Notification.permission;
	const developMode = app.transientSettings.ui._developMode;

	function syncSettings() {
		app.transientSettings.save();
	}

	function updateNotificationSetting() {
		browserNotificationPermission = Notification.permission;
	}

	async function enableBrowserNotifications() {
		await Notification.requestPermission();
		updateNotificationSetting();
	}
</script>

<TabSlot title="App">
	<KeyValue label="Ask before closing">
		<input
			type="checkbox"
			class="checkbox-switch is-info"
			bind:checked={app.transientSettings.app.askBeforeClosing}
			on:change={syncSettings} />
	</KeyValue>
	<KeyValue label="Developer Mode">
		<input
			type="checkbox"
			class="checkbox-switch is-info"
			bind:checked={$developMode}
			on:change={syncSettings} />
	</KeyValue>
	{#if browserNotificationPermission === "default"}
		<KeyValue label="">
			<button class="button is-warning" on:click={enableBrowserNotifications}>Enable browser notifications</button>
		</KeyValue>
	{:else if browserNotificationPermission === "denied"}
		<article class="message is-warning">
			<div class="message-header">
				<p>Notifications</p>
				<button class="delete" aria-label="delete" on:click={updateNotificationSetting}></button>
			</div>
			<div class="message-body">
				Your browser blocked notifications for this page. If you want to use them, enable notifications in your browser settings and close this message.
			</div>
		</article>
	{/if}
</TabSlot>
