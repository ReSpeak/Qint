<script lang="typescript">
	import UiChat from "./chat/UiChat.svelte";
	import UiGlobalSettings from "./panel/UiGlobalSettings.svelte";
	import Searchbar from "./bar/Searchbar.svelte";
	import Sidebar from "./bar/Sidebar.svelte";
	import Toolbar from "./bar/Toolbar.svelte";
	import Description from "./panel/Description.svelte";
	import { DisplayPanel } from "./panel/panel";
	import { app } from "./app";
	import Connect from "./connect/Connect.svelte";
	import GlobalCss from "./GlobalCss.svelte";
	import { ConnectData } from "./connect/connect";

	const connections = app.connections;
	let filter: string = "";

	const chat = app.chat;
	const selected = app.selectedNode;
	const ui = app.transientSettings.ui;

	let showSidebar = ui.showSidebar;
	let showDescription = ui.showDescription;
	let displayPanel = DisplayPanel.Connect;
	let columnStyle = "";
	let connectData = new ConnectData("", "");

	$: {
		columnStyle = "";
		if (showSidebar) columnStyle += " var(--channel-tree-width)";
		else columnStyle += " 0";
		columnStyle += " 1fr";
	}

	function showConnect(data: ConnectData) {
		connectData = data;
		displayPanel = DisplayPanel.Connect;
	}
</script>

<div class="connected-container" style="grid-template-columns: {columnStyle}">
	<!-- TODO Toolbar does not need connection -->
	<Toolbar bind:showSidebar bind:showDescription bind:displayPanel />
	<Searchbar bind:filter visible={showSidebar} />
	<Sidebar {connections} {filter} visible={showSidebar} {showConnect} />
	<div class="panel">
		{#if displayPanel === DisplayPanel.Main}
			<UiChat {chat} />
			{#if showDescription}
				<Description selected={$selected} />
			{/if}
		{:else if displayPanel === DisplayPanel.Settings && $connections.length !== 0}
			<!-- TODO consider something better ? -->
			<UiGlobalSettings connection={$connections[0]} />
		{:else if displayPanel === DisplayPanel.Connect}
			<Connect data={connectData} />
		{/if}
	</div>
</div>
<GlobalCss />

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
		}
	}
</style>
