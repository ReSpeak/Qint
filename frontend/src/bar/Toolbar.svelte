<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { DisplayPanel } from "../panel/panel";
	import { SERVER_ICON } from "../util";
	import { app, NodeSelection } from "../app";
	import ConnectionSettings from "./ConnectionSettings.svelte";

	export let displayPanel: DisplayPanel;
	export let showSidebar: boolean;

	const cons = app.connections;

	function toggleSidebar(show: boolean) {
		showSidebar = show;
	}

	const selectedNode = app.selectedNode;
	$: selectedNodeChanged($selectedNode);
	function selectedNodeChanged(node: NodeSelection | undefined) {
		if (node !== undefined) {
			displayPanel = DisplayPanel.Main;
		}
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
	</div>
	<div class="spacer" />
	<div class="centerButtons">
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
		{#if $cons.length > 0}
			<ConnectionSettings connection={$cons[0]} />
		{/if}
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
</style>
