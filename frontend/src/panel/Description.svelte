<script lang="typescript">
	import DescriptionClient from "./DescriptionClient.svelte";
	import DescriptionChannel from "./DescriptionChannel.svelte";
	import DescriptionServer from "./DescriptionServer.svelte";
	import { NodeSelection } from "../app";

	export let selected: NodeSelection | undefined;
</script>

<div class="description">
	{#if selected !== undefined}
		{#if 'Client' in selected.target}
			<DescriptionClient connection={selected.connection} clientId={selected.target.Client} />
		{:else if 'Channel' in selected.target}
			<DescriptionChannel connection={selected.connection} channelId={selected.target.Channel} />
		{:else if 'Server' in selected.target}
			<DescriptionServer connection={selected.connection} />
		{/if}
	{/if}
</div>

<style lang="scss">
	.description {
		overflow-y: hidden;
		overflow-x: hidden;
		background-color: #242424;
		box-shadow: -3px 0 3px #0005;

		:global(.dataLine) {
			display: flex;
			align-items: center;
			//flex-wrap: nowrap;

			> :global(:first-child) {
				margin-right: 1em;
			}
		}

		:global(.headLine) {
			font-weight: bold;
		}

		:global(.descGroup) {
			padding: 1em;
		}
	}
</style>
