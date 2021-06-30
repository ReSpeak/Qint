<script lang="ts">
	import { app } from "../app";
	import SettingsApp from "./settings/SettingsApp.svelte";
	import SettingsAudio from "./settings/SettingsAudio.svelte";
	import SettingsTts from "./settings/SettingsTts.svelte";
	import SettingsHotkeys from "./settings/SettingsHotkeys.svelte";
	import SettingsIdentities from "./settings/SettingsIdentities.svelte";
	import SettingsNotifications from "./settings/SettingsNotifications.svelte";
	import SettingsPlugins from "./settings/SettingsPlugins.svelte";
	import SettingsDeveloper from "./settings/SettingsDeveloper.svelte";
	import TabList from "../ui/container/TabList.svelte";
	import Icon from "../ui/icon/Icon.svelte";
	import { DisplayPanel } from "../panel/panel";

	const developMode = app.transientSettings.ui._developMode;
</script>

<div class="settings">
	<button
		class="toolbutton is-small compact active"
		on:click={() => app.displayPanel.set(DisplayPanel.Main)}>
		<Icon name="close" />
	</button>
	<TabList _style="height:100%;">
		<SettingsApp />
		<SettingsAudio />
		<SettingsTts />
		<SettingsHotkeys />
		<SettingsIdentities />
		<SettingsNotifications />
		<SettingsPlugins />
		{#if $developMode}
			<SettingsDeveloper />
		{/if}
	</TabList>
</div>

<style lang="scss">
	.settings {
		padding: 1em;
	}

	.toolbutton {
		float: right;
	}
</style>
