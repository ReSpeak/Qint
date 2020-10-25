<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { DisplayPanel } from "../panel/panel";
	import { SERVER_ICON } from "../util";
	import { app, NodeSelection } from "../app";
	import type { Writable } from "svelte/store";
	import { Client } from "../book";
	import type { OChangeConnectionClientUpdate } from "../book_events";

	export let displayPanel: DisplayPanel;
	export let showSidebar: boolean;

	let inputMuted = false;
	let outputMuted = false;
	let isAway = false;

	let showMuteButtons = false;

	const cons = app.connections;
	let ownClient: Writable<Client | undefined> | undefined;
	$: {
		const consVal = $cons;
		showMuteButtons = consVal.length > 0;

		const connection = consVal.length > 0 ? consVal[0] : undefined;
		if (connection !== undefined) {
			ownClient = connection.book.ownClient;
			inputMuted = $ownClient?.inputMuted ?? false;
			outputMuted = $ownClient?.outputMuted ?? false;
			const awayMessage = $ownClient?.awayMessage;
			isAway = awayMessage !== undefined && awayMessage !== null;
		}
	}

	function changeOwnClient(change: OChangeConnectionClientUpdate) {
		for (let c of $cons) {
			c.sendMessage({
				Change: change,
			});
		}
	}

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
			on:click={() => toggleSidebar(!showSidebar)}>
			<Icon name="file-tree" />
		</button>
	</div>
	<div class="spacer" />
	<div class="centerButtons">
		<button
			class="toolbutton"
			class:active={displayPanel === DisplayPanel.Main}
			on:click={() => (displayPanel = DisplayPanel.Main)}>
			<Icon name="chat-outline" />
		</button>
		<button
			class="toolbutton"
			class:active={displayPanel === DisplayPanel.Settings}
			on:click={() => (displayPanel = DisplayPanel.Settings)}>
			<Icon name="cog" />
		</button>
		<button
			class="toolbutton"
			class:active={displayPanel === DisplayPanel.Connect}
			on:click={() => (displayPanel = DisplayPanel.Connect)}>
			<Icon name={SERVER_ICON} />
		</button>
	</div>
	<div class="spacer" />
	<div class="rightButtons">
		<button
			class="toolbutton"
			class:active={inputMuted}
			class:invisible={!showMuteButtons}
			on:click={() => changeOwnClient({ ClientUpdate: { inputMuted: !inputMuted }})}>
			<Icon name={inputMuted ? 'microphone-off' : 'microphone'} />
		</button>
		<button
			class="toolbutton"
			class:active={outputMuted}
			class:invisible={!showMuteButtons}
			on:click={() => changeOwnClient({ ClientUpdate: { outputMuted: !outputMuted }})}>
			<Icon name={outputMuted ? 'volume-off' : 'volume-high'} />
		</button>
		<button
			class="toolbutton"
			class:active={isAway}
			class:invisible={!showMuteButtons}
			on:click={() => changeOwnClient({ ClientUpdate: { away: isAway ? null : '' }})}>
			<Icon name={isAway ? 'sleep' : 'sleep-off'} />
		</button>
		<div style="width: 2em;" />
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
		@extend %unselectable;
		-moz-appearance: none;
		-webkit-appearance: none;

		height: 2.5em;
		border-radius: 100%;
		border: none;
		margin: 0.2em;
		font-size: 1rem;
		display: inline-flex;
		align-items: center;

		background-color: #444444;
		color: #fff;

		cursor: pointer;

		&.active {
			background-color: #888888;
		}
	}
</style>
