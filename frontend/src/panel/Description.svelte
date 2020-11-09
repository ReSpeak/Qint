<script lang="typescript">
	import DescriptionClient from "./DescriptionClient.svelte";
	import DescriptionChannel from "./DescriptionChannel.svelte";
	import DescriptionServer from "./DescriptionServer.svelte";
	import { NodeSelection } from "../app";
	import { Channel, Client, Server } from "../book";

	export let selected: NodeSelection | undefined;
</script>

{#if selected !== undefined}
	<!-- TODO: Remove '!== undefined' when svelte-tool understands it -->
	{#if selected !== undefined && selected.node instanceof Client}
		<DescriptionClient connection={selected.connection} clientId={selected.node.id} />
	{:else if selected !== undefined && selected.node instanceof Channel}
		<DescriptionChannel connection={selected.connection} channel={selected.node} />
	{:else if selected !== undefined && selected.node instanceof Server}
		<DescriptionServer connection={selected.connection} />
	{/if}
{/if}

<style lang="scss">
	:global(.dataLine) {
		display: flex;
		align-items: center;
		//flex-wrap: nowrap;

		> :global(:first-child) {
			margin-right: 1em;
		}
	}

	:global(.descTable) {
		display: grid;
		grid-template-columns: max-content max-content;
		gap: 0 1em;
	}

	:global(.headLine) {
		font-weight: bold;
	}

	:global(.descGroup) {
		padding: 1em;
	}
</style>
