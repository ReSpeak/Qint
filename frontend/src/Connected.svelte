<script lang="typescript">
	import UiChat from "./chat/UiChat.svelte";
	import UiGlobalSettings from "./panel/UiGlobalSettings.svelte";
	import Searchbar from "./bar/Searchbar.svelte";
	import Sidebar from "./bar/Sidebar.svelte";
	import Toolbar from "./bar/Toolbar.svelte";
	import Description from "./panel/Description.svelte";
	import { Connection } from "./connection";

	export let connection: Connection;
	let filter!: string;

	let showSidebar = true;
	let showChat = true;
	let showDescription = true;
	let showGlobalSettings = false;
	let columnStyle = "";
	let panelStyle = "";

	$: globalSettingsChanged(showGlobalSettings);
	$: chatChanged(showChat, showDescription);

	$: {
		columnStyle = "";
		if (showSidebar) columnStyle += " var(--channel-tree-width)";
		else columnStyle += "";
		columnStyle += " 1fr";
	}

	function globalSettingsChanged(showGlobalSettings: boolean) {
		if (showGlobalSettings) {
			showChat = false;
			showDescription = false;
		}
	}

	function chatChanged(showChat: boolean, showDescription: boolean) {
		if (showChat || showDescription) {
			showGlobalSettings = false;
		}
	}
</script>

<div class="connected-container" style="grid-template-columns: {columnStyle}">
	<Toolbar
		{connection}
		bind:showSidebar
		bind:showChat
		bind:showGlobalSettings
		bind:showDescription />
	{#if showSidebar}
		<Searchbar bind:filter />
		<Sidebar {connection} {filter} />
	{/if}
	<div class="panel">
		{#if showGlobalSettings}
			<UiGlobalSettings {connection} />
		{/if}
		{#if showChat}
			<UiChat {connection} />
		{/if}
		{#if showDescription}
			<Description {connection} />
		{/if}
	</div>
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

	.connected-container > .panel {
		grid-row: 2;
		grid-column: 2;
	}

	.panel {
		display: flex;
		flex-direction: row;
		overflow: hidden;

		> :global(*) {
			flex: 1;

			&:not(:last-child) {
				border-right: rgb(179, 179, 179) 2px solid;
			}
		}
	}
</style>
