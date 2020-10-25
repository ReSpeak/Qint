<script lang="typescript">
	import UiChat from "./chat/UiChat.svelte";
	import UiGlobalSettings from "./panel/UiGlobalSettings.svelte";
	import Searchbar from "./bar/Searchbar.svelte";
	import Sidebar from "./bar/Sidebar.svelte";
	import Toolbar from "./bar/Toolbar.svelte";
	import Description from "./panel/Description.svelte";
	import { DisplayPanel } from "./panel/panel";
	import { app } from "./app";
	import Connect from "./connect/UiConnect.svelte";
	import GlobalCss from "./GlobalCss.svelte";
	import { ConnectData } from "./connect/connect";
	import { DescriptionMode } from "./transientSettings";

	const connections = app.connections;
	let filter: string = "";

	const chat = app.chat;
	const selected = app.selectedNode;
	const ui = app.transientSettings.ui;
	let showSidebar = app.showSidebar;
	let displayPanel = app.displayPanel;

	let descriptionMode = ui._descriptionMode;
	let columnStyle = "";
	let connectData = new ConnectData("", "");

	$: {
		columnStyle = "";
		if ($showSidebar) columnStyle += " var(--channel-tree-width)";
		else columnStyle += " 0";
		columnStyle += " 1fr";
	}

	function showConnect(data: ConnectData) {
		connectData = data;
		$displayPanel = DisplayPanel.Connect;
	}
</script>

<div class="appContainer" style="grid-template-columns: {columnStyle}">
	<Toolbar bind:showSidebar={$showSidebar} bind:displayPanel={$displayPanel} />
	<Searchbar bind:filter visible={$showSidebar} />
	<Sidebar {connections} {filter} visible={$showSidebar} {showConnect} />
	<div class="panel">
		{#if $displayPanel === DisplayPanel.Main}
			<UiChat {chat} />
			{#if $descriptionMode !== DescriptionMode.None}
				<Description selected={$selected} />
			{/if}
		{:else if $displayPanel === DisplayPanel.Settings && $connections.length !== 0}
			<!-- TODO consider something better ? -->
			<UiGlobalSettings connection={$connections[0]} />
		{:else if $displayPanel === DisplayPanel.Connect}
			<Connect data={connectData} />
		{/if}
	</div>
</div>
<GlobalCss />

<style lang="scss">
	.appContainer {
		display: grid;
		grid-template-rows: max-content 1fr;

		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		left: 0;
		height: 100%;

		> :global(.toolbar) {
			grid-row: 1;
			grid-column: 2;
		}
		> :global(.searchbar) {
			grid-row: 1;
			grid-column: 1;
		}
		> :global(.sidebar) {
			grid-row: 2;
			grid-column: 1;
		}
		> .panel {
			grid-row: 2;
			grid-column: 2;
		}
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
