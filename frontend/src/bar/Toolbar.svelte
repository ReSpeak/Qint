<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { DisplayPanel } from "../panel/panel";
	import { SERVER_ICON } from "../util";
	import { app, NodeSelection } from "../app";
	import type { Writable } from "svelte/store";
	import { Client } from "../book";

	export let showSidebar: boolean;
	export let showDescription: boolean;
	export let displayPanel: DisplayPanel;

	let inputMuted = false;
	let outputMuted = false;
	let isAway = false;

	let showDescriptionButton = false;

	const cons = app.connections;
	let ownClient: Writable<Client | undefined> | undefined;
	$: {
		const consVal = $cons;
		showDescriptionButton = consVal.length > 0;
		if (!showDescriptionButton) showDescription = false;

		const connection = consVal.length > 0 ? consVal[0] : undefined;
		if (connection !== undefined) {
			ownClient = connection.book.ownClient;
			inputMuted = $ownClient?.input_muted ?? false;
			outputMuted = $ownClient?.output_muted ?? false;
			const awayMessage = $ownClient?.away_message;
			isAway = awayMessage !== undefined && awayMessage !== null;
		}
	}

	function changeOwnClient(change: any) {
		for (let c of $cons) {
			c.sendMessage({
				Change: {
					ClientUpdate: change,
				},
			});
		}
	}

	$: displayPanelChanged(displayPanel);
	function displayPanelChanged(pan: DisplayPanel) {
		if (pan !== DisplayPanel.Main) showDescription = false;
	}

	$: showDescriptionChanged(showDescription);
	function showDescriptionChanged(to: boolean) {
		if (to) displayPanel = DisplayPanel.Main;
	}

	const selectedNode = app.selectedNode;
	$: selectedNodeChanged($selectedNode);
	function selectedNodeChanged(node: NodeSelection | undefined) {
		if (node !== undefined) displayPanel = DisplayPanel.Main;
	}
</script>

<div class="toolbar">
	<div class="leftButtons">
		<button
			class="button toolbutton"
			class:active={showSidebar}
			on:click={() => (showSidebar = !showSidebar)}>
			<Icon name="file-tree" />
		</button>
	</div>
	<div class="spacer" />
	<div class="centerButtons">
		<button
			class="button toolbutton"
			class:active={displayPanel === DisplayPanel.Main}
			on:click={() => (displayPanel = DisplayPanel.Main)}>
			<Icon name="chat-outline" />
		</button>
		<button
			class="button toolbutton"
			class:active={displayPanel === DisplayPanel.Settings}
			on:click={() => (displayPanel = DisplayPanel.Settings)}>
			<Icon name="cog" />
		</button>
		<button
			class="button toolbutton"
			class:active={displayPanel === DisplayPanel.Connect}
			on:click={() => (displayPanel = DisplayPanel.Connect)}>
			<Icon name={SERVER_ICON} />
		</button>
	</div>
	<div class="spacer" />
	<div class="rightButtons" class:invisible={!showDescriptionButton}>
		<button
			class="button toolbutton"
			class:active={inputMuted}
			on:click={() => changeOwnClient({ input_muted: !inputMuted })}>
			<Icon name={inputMuted ? 'microphone-off' : 'microphone'} />
		</button>
		<button
			class="button toolbutton"
			class:active={outputMuted}
			on:click={() => changeOwnClient({ output_muted: !outputMuted })}>
			<Icon name={outputMuted ? 'volume-off' : 'volume-high'} />
		</button>
		<button
			class="button toolbutton"
			class:active={isAway}
			on:click={() => changeOwnClient({ away: isAway ? null : '' })}>
			<Icon name={isAway ? 'sleep' : 'sleep-off'} />
		</button>
		<div style="width: 2em;" />
		<button
			class="button toolbutton"
			class:active={showDescription}
			on:click={() => (showDescription = !showDescription)}>
			<Icon name="information-outline" />
		</button>
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

	.toolbutton {
		background-color: #444444;
		border-radius: 100%;
		border: none;
		margin: 0.2em;

		&:focus {
			box-shadow: none;
		}

		&.active {
			background-color: #888888;
		}
	}
</style>
