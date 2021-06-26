<script lang="ts">
	import { app } from "../app";
	import SettingsApp from "./settings/SettingsApp.svelte";
	import SettingsAudio from "./settings/SettingsAudio.svelte";
	import SettingsTts from "./settings/SettingsTts.svelte";
	import SettingsHotkeys from "./settings/SettingsHotkeys.svelte";
	import SettingsIdentities from "./settings/SettingsIdentities.svelte";
	import SettingsNotifications from "./settings/SettingsNotifications.svelte";
	import SettingsDeveloper from "./settings/SettingsDeveloper.svelte";
	import BTabList from "../ui/BTabList.svelte";
	import Icon from "../ui/Icon.svelte";
	import { DisplayPanel } from "../panel/panel";

	const developMode = app.transientSettings.ui._developMode;
	const displayPanel = app.displayPanel;
</script>

<div class="settings">
	<button
		class="toolbutton is-small compact active"
		on:click={() => ($displayPanel = DisplayPanel.Main)}>
		<Icon name="close" />
	</button>
	<BTabList _style="height:100%;">
		<SettingsApp />
		<SettingsAudio />
		<SettingsTts />
		<SettingsHotkeys />
		<SettingsIdentities />
		<SettingsNotifications />
		{#if $developMode}
			<SettingsDeveloper />
		{/if}
	</BTabList>
</div>

<style lang="scss">
	.settings {
		padding: 1em;
	}

	.toolbutton {
		float: right;
	}
</style>
