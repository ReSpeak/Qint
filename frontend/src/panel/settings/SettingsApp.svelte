<script lang="ts">
	import { app } from "../../app";
	import BTabSlot from "../../ui/BTabSlot.svelte";
	import BKeyValue from "../../ui/BKeyValue.svelte";

	const browserNotificationDenied = Notification.permission === "denied";
	const developMode = app.transientSettings.ui._developMode;

	function syncSettings() {
		app.transientSettings.save();
	}

	async function browserNotificationChanged() {
		syncSettings();
		if (
			app.transientSettings.app.allowBrowserNotifications &&
			Notification.permission === "default"
		) {
			await Notification.requestPermission();
		}
	}
</script>

<BTabSlot title="App">
	<BKeyValue label="Ask before closing">
		<input
			type="checkbox"
			class="checkbox-switch is-info"
			bind:checked={app.transientSettings.app.askBeforeClosing}
			on:change={() => syncSettings()} />
	</BKeyValue>
	<BKeyValue label="Developer Mode">
		<input
			type="checkbox"
			class="checkbox-switch is-info"
			bind:checked={$developMode}
			on:change={() => syncSettings()} />
	</BKeyValue>
	<BKeyValue
		label="Browser notifications"
		title={browserNotificationDenied
			? "Your browser blocked notifications for this page. If you want to use them, enable notifications in your browser settings and reload the page."
			: ""}>
		<input
			type="checkbox"
			class="checkbox-switch is-info"
			disabled={browserNotificationDenied}
			bind:checked={app.transientSettings.app.allowBrowserNotifications}
			on:change={browserNotificationChanged} />
	</BKeyValue>
</BTabSlot>
