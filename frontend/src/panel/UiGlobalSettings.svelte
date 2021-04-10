<script lang="typescript">
	import SettingsApp from "./settings/SettingsApp.svelte";
	import SettingsAudio from "./settings/SettingsAudio.svelte";
	import SettingsTts from "./settings/SettingsTts.svelte";
	import SettingsHotkeys from "./settings/SettingsHotkeys.svelte";
	import BTabList from "../ui/BTabList.svelte";

	let tablistIndex: number;
</script>

<div class="settings">
	<BTabList bind:activeIndex={tablistIndex}>
		<SettingsApp />
		<SettingsAudio />
		<SettingsTts />
		<SettingsHotkeys />
	</BTabList>
</div>

<style lang="scss">
	.settings {
		padding: 1em;
	}
</style>
