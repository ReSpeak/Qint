<script lang="typescript">
	import { Connection } from "../connection";
	import DescriptionClient from "./DescriptionClient.svelte";
	import DescriptionChannel from "./DescriptionChannel.svelte";

	export let connection: Connection;
	let selected = connection.chat.selectedChat;
	$: s = $selected;
</script>

<div class="description">
	{#if 'Client' in s}
		<DescriptionClient {connection} clientId={s.Client} />
	{:else if 'Channel' in s}
		<DescriptionChannel {connection} channelId={s.Channel} />
	{:else if 'Server' in s}
		<div>Server here</div>
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
