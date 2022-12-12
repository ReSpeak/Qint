<script lang="ts">
	import { IS_TAURI } from "../../util";
	import { app } from "../../app";
	import TabSlot from "../../ui/container/TabSlot.svelte";
	import KeyValue from "../../ui/util/KeyValue.svelte";
	import { TitleBarStyle } from "../../settings";

	let browserNotificationPermission = Notification.permission;
	const developMode = app.settings.ui._developMode;
	const appSettings = app.settings.app;
	const titleBarStyle = appSettings._titleBarStyle;

	function syncSettings() {
		app.settings.save();
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
			bind:checked={appSettings.askBeforeClosing}
			on:change={syncSettings}
		/>
	</KeyValue>
	{#if IS_TAURI}
		<KeyValue label="Minimize to Tray">
			<input
				type="checkbox"
				class="checkbox-switch is-info"
				bind:checked={appSettings.minimizeToTray}
				on:change={syncSettings}
			/>
		</KeyValue>
		<KeyValue label="Close to Tray">
			<input
				type="checkbox"
				class="checkbox-switch is-info"
				bind:checked={appSettings.closeToTray}
				on:change={syncSettings}
			/>
		</KeyValue>
		<KeyValue label="Window Design" autoLabel={false}>
			<div>
				<input
					type="radio"
					id="tb1"
					name="titleBarStyle"
					bind:group={$titleBarStyle}
					value={TitleBarStyle.Native}
					on:change={syncSettings}
				/>
				<label for="tb1">Native</label>
			</div>

			<div>
				<input
					type="radio"
					id="tb2"
					name="titleBarStyle"
					bind:group={$titleBarStyle}
					value={TitleBarStyle.Normal}
					on:change={syncSettings}
				/>
				<label for="tb2">Normal</label>
			</div>
		</KeyValue>
	{/if}
	<KeyValue label="Developer Mode">
		<input
			type="checkbox"
			class="checkbox-switch is-info"
			bind:checked={$developMode}
			on:change={syncSettings}
		/>
	</KeyValue>
	{#if !IS_TAURI}
		{#if browserNotificationPermission === "default"}
			<KeyValue label="">
				<button class="button is-warning" on:click={enableBrowserNotifications}>
					Enable browser notifications
				</button>
			</KeyValue>
		{:else if browserNotificationPermission === "denied"}
			<article class="message is-warning">
				<div class="message-header">
					<p>Notifications</p>
					<button
						class="delete"
						aria-label="delete"
						on:click={updateNotificationSetting}
					/>
				</div>
				<div class="message-body">
					Your browser blocked notifications for this page. If you want to use them,
					enable notifications in your browser settings and close this message.
				</div>
			</article>
		{/if}
	{/if}
</TabSlot>
