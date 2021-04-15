<script lang="ts">
	import SettingsApp from "./settings/SettingsApp.svelte";
	import SettingsAudio from "./settings/SettingsAudio.svelte";
	import SettingsTts from "./settings/SettingsTts.svelte";
	import SettingsHotkeys from "./settings/SettingsHotkeys.svelte";
	import SettingsIdentities from "./settings/SettingsIdentities.svelte";
	import BTabList from "../ui/BTabList.svelte";
</script>

<div class="settings">
	<BTabList _style="height:100%;">
		<SettingsApp />
		<SettingsAudio />
		<SettingsTts />
		<SettingsHotkeys />
		<SettingsIdentities />
	</BTabList>
</div>

<style lang="scss">
	.settings {
		padding: 1em;
	}
</style>
