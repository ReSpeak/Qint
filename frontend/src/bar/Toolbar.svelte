<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { Connection } from "../connection";
	import { DisplayPanel } from "../panel/panel";

	export let connection: Connection;
	export let showSidebar: boolean;
	export let showDescription: boolean;
	export let displayPanel: DisplayPanel;

	let ownClient = connection.ownClient;
	$: input_muted = $ownClient?.input_muted;
	$: output_muted = $ownClient?.output_muted;
	$: isAway = $ownClient?.away_message !== null;

	function changeOwnClient(change: any) {
		connection.sendMessage({
			Change: {
				ClientUpdate: change,
			},
		});
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
	</div>
	<div class="spacer" />
	<div class="rightButtons">
		<button
			class="button toolbutton"
			class:active={input_muted}
			on:click={() => changeOwnClient({ input_muted: !input_muted })}>
			<Icon name={input_muted ? 'microphone-off' : 'microphone'} />
		</button>
		<button
			class="button toolbutton"
			class:active={output_muted}
			on:click={() => changeOwnClient({ output_muted: !output_muted })}>
			<Icon name={output_muted ? 'volume-off' : 'volume-high'} />
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
