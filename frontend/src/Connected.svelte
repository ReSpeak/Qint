<script lang="typescript">
	import UiChat from "./chat/UiChat.svelte";
	import UiGlobalSettings from "./panel/UiGlobalSettings.svelte";
	import Searchbar from "./bar/Searchbar.svelte";
	import Sidebar from "./bar/Sidebar.svelte";
	import Toolbar from "./bar/Toolbar.svelte";
	import Description from "./panel/Description.svelte";
	import { Connection } from "./connection";
	import { DisplayPanel } from "./panel/panel";
	import { transientSettings } from "./transientSettings";

	export let connection: Connection;
	let filter: string = "";

	const ui = transientSettings.ui;

	let showSidebar = ui.showSidebar;
	let showDescription = ui.showDescription;
	let displayPanel = DisplayPanel.Main;
	let columnStyle = "";

	$: {
		ui.showSidebar = showSidebar;
		ui.showDescription = showDescription;
		transientSettings.sync_to_proxy();
	}

	$: {
		columnStyle = "";
		if (showSidebar) columnStyle += " var(--channel-tree-width)";
		else columnStyle += " 0";
		columnStyle += " 1fr";
	}
</script>

<div class="connected-container" style="grid-template-columns: {columnStyle}">
	<Toolbar {connection} bind:showSidebar bind:showDescription bind:displayPanel />
	<Searchbar bind:filter visible={showSidebar} />
	<Sidebar {connection} {filter} visible={showSidebar} />
	<div class="panel">
		{#if displayPanel === DisplayPanel.Main}
			<UiChat {connection} />
			{#if showDescription}
				<Description {connection} />
			{/if}
		{:else if displayPanel === DisplayPanel.Settings}
			<UiGlobalSettings {connection} />
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
		}
	}
</style>
