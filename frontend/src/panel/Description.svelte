<script lang="ts">
	import DescriptionClient from "./DescriptionClient.svelte";
	import DescriptionOfflineClient from "./DescriptionOfflineClient.svelte";
	import DescriptionChannel from "./DescriptionChannel.svelte";
	import DescriptionServer from "./DescriptionServer.svelte";
	import DescriptionOfflineServer from "./DescriptionOfflineServer.svelte";
	import DescriptionMultiSelection from "./DescriptionMultiSelection.svelte";
	import { NodeSelections } from "../app";
	import { Channel, Client, GraphQlClient, GraphQlServer, Server } from "../book";

	export let selected: NodeSelections;
	export let editing: boolean;

	$: selection = selected.getSingleSelection();
</script>

{#if selection !== undefined}
	{#if selection.node instanceof Client && selection.connection !== undefined}
		<DescriptionClient connection={selection.connection} client={selection.node} bind:editing />
	{:else if selection.node instanceof GraphQlClient}
		<DescriptionOfflineClient client={selection.node} bind:editing />
	{:else if selection.node instanceof Channel && selection.connection !== undefined}
		<DescriptionChannel
			connection={selection.connection}
			channel={selection.node}
			bind:editing />
	{:else if selection.node instanceof Server && selection.connection !== undefined}
		<DescriptionServer connection={selection.connection} server={selection.node} bind:editing />
	{:else if selection.node instanceof GraphQlServer}
		<DescriptionOfflineServer server={selection.node} bind:editing />
	{/if}
{:else if selected.selections.length !== 0}
	<DescriptionMultiSelection {selected} />
{/if}

<style lang="scss">
	:global(.dataLine) {
		display: flex;
		align-items: center;
		//flex-wrap: nowrap;
		margin-bottom: 0.5em;

		> :global(:first-child) {
			margin-right: 1em;
		}
	}

	// Multiline text areas
	:global(.dataLine.large.editing) {
		align-items: inherit;
		display: grid;
		grid-template-columns: auto 1fr;
		grid-gap: 0.5em;

		:global(.editbox) {
			grid-area: 1 / 2 / 3 / 2;
		}
	}

	:global(.descTable) {
		display: grid;
		grid-template-columns: max-content max-content;
		gap: 0.5em 1em;
		align-items: center;
	}

	:global(.headLine) {
		font-weight: bold;
	}

	:global(.descGroup) {
		padding: 1em;
		white-space: nowrap;

		:global(&.editing) > :global(*:not(:last-child)) {
			margin-bottom: 0.5em;
		}
	}
</style>
