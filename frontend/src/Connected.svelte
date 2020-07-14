<script lang="typescript">
	import UiChat from "./chat/UiChat.svelte";
	import UiGlobalSettings from "./settings/UiGlobalSettings.svelte";
	import Searchbar from "./bar/Searchbar.svelte";
	import Sidebar from "./bar/Sidebar.svelte";
	import Toolbar from "./bar/Toolbar.svelte";
	import { Connection } from "./connection";

	export let connection: Connection;
	let filter!: string;

	let showSidebar = true;
	let showChat = true;
	let showGlobalSettings = false;
	let columnStyle = "";

	$: globalSettingsChanged(showGlobalSettings);
	$: chatChanged(showChat);

	$: {
		columnStyle = "";
		if (showSidebar)
			columnStyle += " var(--channel-tree-width)";
		else
			columnStyle += " 0";
		if (showChat)
			columnStyle += " 1fr";
	}

	function globalSettingsChanged(showGlobalSettings: boolean) {
		if (showGlobalSettings) {
			showChat = false;
		}
	}

	function chatChanged(showChat: boolean) {
		if (showChat) {
			showGlobalSettings = false;
		}
	}
</script>

<div class="connected-container" style="grid-template-columns: {columnStyle}">
	<Toolbar {connection} bind:showSidebar bind:showChat bind:showGlobalSettings />
	{#if showSidebar}
		<Searchbar bind:filter/>
		<Sidebar {connection} {filter}/>
	{/if}
	{#if showGlobalSettings}
		<UiGlobalSettings {connection}/>
	{/if}
	{#if showChat}
		<UiChat {connection}/>
	{/if}
</div>

<style lang="scss">
	.connected-container {
		display: grid;
		grid-template-rows: max-content 1fr;

		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		left: 0;
	}

	.connected-container > :global(.toolbar) {
		grid-row: 1;
		grid-column: 2;
	}

	.connected-container > :global(.searchbar) {
		grid-row: 1;
		grid-column: 1;
	}

	.connected-container > :global(.sidebar) {
		grid-row: 2;
		grid-column: 1;
	}

	.connected-container > :global(.chat) {
		grid-row: 2;
		grid-column: 2;
	}
</style>
