<script lang="ts">
	import Icon from "../ui/icon/Icon.svelte";
	import { DisplayPanel } from "../panel/panel";
	import { SERVER_ICON } from "../util";
	import { app, NodeSelections } from "../app";
	import ConnectionSettings from "./ConnectionSettings.svelte";
	import { ConnectData } from "../connect/uiConnect";
	import Searchbar from "./Searchbar.svelte";

	export let displayPanel: DisplayPanel;
	export let showSidebar: boolean;
	export let connectData: ConnectData;
	export let filter: string; // from the search

	function toggleSidebar(show: boolean) {
		showSidebar = show;
	}

	const selectedNode = app.selectedNode;
	$: selectedNodeChanged($selectedNode);
	function selectedNodeChanged(node: NodeSelections) {
		if (node.selections.length !== 0) displayPanel = DisplayPanel.Main;
	}
</script>

<div class="toolbar">
	<div class="leftButtons">
		<button
			class="toolbutton"
			class:active={showSidebar}
			on:click={() => toggleSidebar(!showSidebar)}
			title="Channel tree">
			<Icon name="file-tree" />
		</button>
		<div class="searchbar">
			<Searchbar bind:filter visible={true} />
		</div>
	</div>
	<div class="spacer" />
	<div class="centerButtons toolbuttons">
		{#if filter !== ""}
			<button
				class="toolbutton"
				class:active={displayPanel === DisplayPanel.Search}
				on:click={() => (displayPanel = DisplayPanel.Search)}
				title="Chat">
				<Icon name="magnify" />
			</button>
		{/if}
		<button
			class="toolbutton"
			class:active={displayPanel === DisplayPanel.Main}
			on:click={() => (displayPanel = DisplayPanel.Main)}
			title="Chat">
			<Icon name="chat-outline" />
		</button>
		<button
			class="toolbutton"
			class:active={displayPanel === DisplayPanel.Settings}
			on:click={() => (displayPanel = DisplayPanel.Settings)}
			title="Settings">
			<Icon name="cog" />
		</button>
		<button
			class="toolbutton"
			class:active={displayPanel === DisplayPanel.Connect}
			on:click={() => (displayPanel = DisplayPanel.Connect)}
			title="Connect to a new server">
			<Icon name={SERVER_ICON} />
		</button>
	</div>
	<div class="spacer" />
	<div class="rightButtons">
		<ConnectionSettings bind:connectData />
	</div>
</div>

<style lang="scss">
	.toolbar {
		background-color: $box-background-color;
		padding: 0.5em;
		display: flex;
	}

	.spacer {
		flex: 1;
	}

	.centerButtons,
	.leftButtons,
	.rightButtons {
		display: inline-flex;
	}

	.searchbar {
		padding-left: 1em;
	}
</style>
