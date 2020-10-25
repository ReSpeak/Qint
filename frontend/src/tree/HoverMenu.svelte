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
			infoActive = $descriptionMode === DescriptionMode.Info;
			filesActive = $descriptionMode === DescriptionMode.Files;
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
	}
</script>

<div class="hover menu" style="top: {div.getBoundingClientRect().top}px;">
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
			on:click={() => setDescriptionMode(DescriptionMode.Info)}>
			<Icon name="information-outline" />
		</button>
		{#if !(selected.node instanceof Client)}
			<button
				class="toolbutton"
				class:active={filesActive}
				on:click={() => setDescriptionMode(DescriptionMode.Files)}>
				<Icon name="folder" />
			</button>
		{/if}
	</div>
</div>

<style lang="scss">
	.hover {
		left: calc(var(--channel-tree-width) - 0em);
		display: grid;
		grid-gap: 1em;
	}

	.hover > :global(.name) {
		grid-row: 1;
		grid-column: 1 / 3;
	}

	.hover :global(.icon) {
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

	.hover .buttons {
		margin-bottom: 0.5em;
	}
</style>
