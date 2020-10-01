<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { DisplayPanel } from "../panel/panel";
	import { SERVER_ICON } from "../util";
	import { app } from "../app";
	import type { Writable } from "svelte/store";
	import { Client } from "../book";

	export let showSidebar: boolean;
	export let showDescription: boolean;
	export let displayPanel: DisplayPanel;

	let inputMuted = false;
	let outputMuted = false;
	let isAway = false;

	const cons = app.connections;
	let ownClient: Writable<Client | undefined> | undefined;
	$: {
		const consVal = $cons;
		const connection = consVal.length > 0 ? consVal[0] : undefined;
		if (connection !== undefined) {
			ownClient = connection.book.ownClient;
			inputMuted = $ownClient?.input_muted ?? false;
			outputMuted = $ownClient?.output_muted ?? false;
			isAway = $ownClient?.away_message !== null ?? false;
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
	<div class="rightButtons">
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
