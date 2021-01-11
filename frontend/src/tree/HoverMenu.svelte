<script lang="typescript">
	import HoverMenuClient from "./HoverMenuClient.svelte";
	import HoverMenuChannel from "./HoverMenuChannel.svelte";
	import HoverMenuServer from "./HoverMenuServer.svelte";
	import Icon from "../ui/Icon.svelte";
	import { app, NodeSelection } from "../app";
	import { Channel, Client, Server } from "../book";
	import { DescriptionMode } from "../transientSettings";
	import HoverContainer from "./HoverContainer.svelte";

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
		if ((mode === DescriptionMode.Info && infoActive) || (mode === DescriptionMode.Files && filesActive)) {
			$descriptionMode = DescriptionMode.None;
		} else {
			$descriptionMode = mode;
			app.selectNode(selected);
		}
		app.transientSettings.save();
	}

	function disconnect() {
		selected.connection.disconnect();
	}
</script>

<HoverContainer {div}>
	{#if selected.node instanceof Client}
		<HoverMenuClient connection={selected.connection} client={selected.node} />
	{:else if selected.node instanceof Channel}
		<HoverMenuChannel connection={selected.connection} channel={selected.node} />
	{:else if selected.node instanceof Server}
		<HoverMenuServer connection={selected.connection} />
	{/if}
	<div class="toolbuttons">
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
		{#if selected.node instanceof Server}
			<button
				class="toolbutton"
				on:click={disconnect}
				title="Disconnect">
				<Icon name="exit-to-app" />
			</button>
		{/if}
	</div>
</HoverContainer>

<style lang="scss">
	:global(.hover) > :global(.description) {
		font-size: 0.85em;
	}
</style>
