<script lang="ts">
	import Chat from "./chat/Chat.svelte";
	import Settings from "./panel/Settings.svelte";
	import Sidebar from "./bar/Sidebar.svelte";
	import Toolbar from "./bar/Toolbar.svelte";
	import Description from "./panel/Description.svelte";
	import FileBrowser from "./panel/FileBrowser.svelte";
	import ServerFileBrowser from "./panel/ServerFileBrowser.svelte";
	import Search from "./search/Search.svelte";
	import { DisplayPanel } from "./panel/panel";
	import { app } from "./app";
	import Connect from "./connect/Connect.svelte";
	import GlobalCss from "./GlobalCss.svelte";
	import GlobalScss from "./GlobalScss.svelte";
	import { ConnectData, MuteState } from "./connect/uiConnect";
	import type { MuteStates } from "./connect/uiConnect";
	import { DescriptionMode } from "./transientSettings";
	import { Channel, Server } from "./book";
	import { backend } from "./backend/backend";
	import { onMount } from "svelte";
	import { derived, writable } from "svelte/store";
	import type { Readable, Writable } from "svelte/store";

	const connections = app.connections;
	let filter: string = "";

	const chat = app.chat;
	const selected = app.selectedNode;
	$: sel = $selected.getSingleSelection();
	const ui = app.transientSettings.ui;
	const showSidebar = app.showSidebar;
	const displayPanel = app.displayPanel;

	const descriptionMode = ui._descriptionMode;
	let columnStyle = "";
	let connectData = new ConnectData("", "");

	$: {
		columnStyle = "";
		if ($showSidebar) columnStyle += " var(--channel-tree-width)";
		else columnStyle += " 0";
		columnStyle += " 1fr";
	}

	$: filterChanged(filter);

	let connectStringDerived: Readable<string>;
	$: {
		if ($connections.length === 0) {
			connectStringDerived = writable("");
		} else {
			connectStringDerived = derived(
				$connections.map((c) => c.connectOptions) as [Writable<ConnectData>],
				(cs) => {
					return JSON.stringify(cs);
				}
			);
		}
	}

	$: location.hash = $connectStringDerived;

	function showConnect(data: ConnectData) {
		connectData = data;
		$displayPanel = DisplayPanel.Connect;
	}

	function filterChanged(filter: string) {
		if (filter !== "") {
			// Show search panel
			$displayPanel = DisplayPanel.Search;
		}
	}

	async function updateGlobalMuteState() {
		try {
			const state: MuteStates = await (await backend.fetch("/mutestate")).json();
			connectData.inputMuted = state.input;
			connectData.outputMuted = state.output;
			connectData.away = state.away ? "" : undefined;

			// Save in transientsettings
			const ui = app.transientSettings.ui;
			let changed = false;
			if (ui.defaultInputMuted !== (connectData.inputMuted !== MuteState.None)) {
				ui.defaultInputMuted = connectData.inputMuted !== MuteState.None;
				changed = true;
			}
			if (ui.defaultOutputMuted !== (connectData.outputMuted !== MuteState.None)) {
				ui.defaultOutputMuted = connectData.outputMuted !== MuteState.None;
				changed = true;
			}
			if (ui.defaultAway !== state.away) {
				ui.defaultAway = state.away;
				changed = true;
			}
			if (changed) app.transientSettings.save();
		} catch (e) {
			console.log("Failed to get mute state", e);
		}
	}

	onMount(() => {
		updateGlobalMuteState();
		const unsub = app.updateMuteState.subscribe(updateGlobalMuteState);
		return unsub;
	});
</script>

<div class="appContainer" style="grid-template-columns: {columnStyle}">
	<Toolbar
		bind:showSidebar={$showSidebar}
		bind:displayPanel={$displayPanel}
		bind:connectData
		bind:filter />

	<Sidebar
		{connections}
		notifications={app.nofifications}
		{filter}
		visible={$showSidebar}
		{showConnect} />
	<div class="displayPanel">
		{#if $displayPanel === DisplayPanel.Main}
			<Chat {chat} />
			{#if $descriptionMode !== DescriptionMode.None}
				<div class="description">
					{#if $descriptionMode === DescriptionMode.Files && sel !== undefined && (sel.node instanceof Channel || sel.node instanceof Server)}
						{#if sel.node instanceof Channel}
							<FileBrowser connection={sel.connection} channelId={sel.node.id} />
						{:else if sel.node instanceof Server}
							<ServerFileBrowser connection={sel.connection} />
						{/if}
					{:else}
						<Description selected={$selected} />
					{/if}
				</div>
			{/if}
		{:else if $displayPanel === DisplayPanel.Settings}
			<Settings />
		{:else if $displayPanel === DisplayPanel.Connect}
			<Connect bind:data={connectData} />
		{:else if $displayPanel === DisplayPanel.Search}
			<Search {filter} />
		{/if}
	</div>
</div>
<GlobalCss />
<GlobalScss />

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
			grid-column: 1 / span 2;
		}
		> :global(.searchbar) {
			grid-row: 1;
			grid-column: 1;
		}
		> :global(.sidebar) {
			grid-row: 2;
			grid-column: 1;
		}
		> .displayPanel {
			grid-row: 2;
			grid-column: 2;
		}
	}

	.displayPanel {
		display: flex;
		flex-direction: row;
		overflow: hidden;

		> :global(*) {
			flex: 1;
		}
	}

	.description {
		overflow-y: hidden;
		overflow-x: hidden;
		background-color: $box-background-color;
		box-shadow: -3px 0 3px #0005;
		display: flex;
		flex-direction: column;
	}
</style>
