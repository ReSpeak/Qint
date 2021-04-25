<script lang="ts">
	import { app } from "../app";
	import SettingsApp from "./settings/SettingsApp.svelte";
	import SettingsAudio from "./settings/SettingsAudio.svelte";
	import SettingsTts from "./settings/SettingsTts.svelte";
	import SettingsHotkeys from "./settings/SettingsHotkeys.svelte";
	import SettingsIdentities from "./settings/SettingsIdentities.svelte";
	import SettingsDeveloper from "./settings/SettingsDeveloper.svelte";
	import BTabList from "../ui/BTabList.svelte";

	const developMode = app.transientSettings.ui._developMode;
</script>

<div class="settings">
	<BTabList _style="height:100%;">
		<SettingsApp />
		<SettingsAudio />
		<SettingsTts />
		<SettingsHotkeys />
		<SettingsIdentities />
		{#if $developMode}
			<SettingsDeveloper />
		{/if}
	</BTabList>
</div>

<style lang="scss">
	.settings {
		padding: 1em;
	}
</style>
