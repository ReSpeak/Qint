<script lang="typescript">
	import HoverMenuClient from "./HoverMenuClient.svelte";
	import HoverMenuChannel from "./HoverMenuChannel.svelte";
	import HoverMenuServer from "./HoverMenuServer.svelte";
	import Icon from "../ui/Icon.svelte";
	import { app, NodeSelection } from "../app";
	import { Channel, Client, Server } from "../book";
	import { DescriptionMode } from "../transientSettings";

	export let div: HTMLElement;
	export let selected: NodeSelection;

	let curSelected = app.selectedNode;
	let descriptionMode = app.transientSettings.ui._descriptionMode;

	let infoActive: boolean;
	let filesActive: boolean;
	$: {
		if (NodeSelection.equals($curSelected, selected)) {
			if (selected.node instanceof Client) {
				infoActive = $descriptionMode !== DescriptionMode.None;
			} else {
				infoActive = $descriptionMode === DescriptionMode.Info;
				filesActive = $descriptionMode === DescriptionMode.Files;
			}
		} else {
			infoActive = false;
			filesActive = false;
		}
	}

	function setDescriptionMode(mode: DescriptionMode) {
		if ((mode === DescriptionMode.Info && infoActive) || (mode == DescriptionMode.Files && filesActive)) {
			$descriptionMode = DescriptionMode.None;
		} else {
			$descriptionMode = mode;
			app.selectNode(selected);
		}
		app.transientSettings.save("ui");
	}
</script>

<div class="hover menu" style="top: calc({div.getBoundingClientRect().top}px - 1.5em);">
	<div class="corner" />
	{#if selected.node instanceof Client}
		<HoverMenuClient connection={selected.connection} clientId={selected.node.id} />
	{:else if selected.node instanceof Channel}
		<HoverMenuChannel connection={selected.connection} channelId={selected.node.id} />
	{:else if selected.node instanceof Server}
		<HoverMenuServer connection={selected.connection} />
	{/if}
	<div class="buttons">
		<button
			class="toolbutton"
			class:active={infoActive}
			on:click={() => setDescriptionMode(DescriptionMode.Info)}
			title="Details">
			<Icon name="information-outline" />
		</button>
		{#if !(selected.node instanceof Client)}
			<button
				class="toolbutton"
				class:active={filesActive}
				on:click={() => setDescriptionMode(DescriptionMode.Files)}
				title="Browse files">
				<Icon name="folder" />
			</button>
		{/if}
	</div>
</div>

<style lang="scss">
	.hover {
		position: fixed;
		z-index: 3;
		border: solid 1px $border;
		border-radius: 0.5em;
		background: $background;
		padding: 0.5em;
		left: var(--channel-tree-width);
		display: flex;
		flex-direction: column;
		gap: 1em;
	}

	.hover .corner {
		position: absolute;
		transform: rotate(45deg);
		left: -0.3em;
		top: 1.8em;
		width: 0.5em;
		height: 0.5em;
		border-left: solid 1px $border;
		border-bottom: solid 1px $border;
		background: $background;
	}

	.buttons :global(.icon) {
		margin-left: unset !important;
		margin-right: unset !important;
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

	.buttons {
		margin-bottom: 0.5em;
	}
</style>
