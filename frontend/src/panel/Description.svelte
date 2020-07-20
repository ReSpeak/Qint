<script lang="typescript">
	import { Connection } from "../connection";
	import DescriptionClient from "./DescriptionClient.svelte";
	import DescriptionChannel from "./DescriptionChannel.svelte";
	import { MessageTarget } from "../structs/ts";

	export let connection!: Connection;
	let selected = connection.chat.selectedChat;
	let s: MessageTarget;
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
		overflow-y: scroll;
		overflow-x: hidden;

		padding: 0.5em;

		:global(.dataLine) {
			display: flex;
			//flex-wrap: nowrap;

			> :global(:first-child) {
				margin-right: 1em;
			}
		}

		:global(.headLine) {
			font-weight: bold;
		}
	}
</style>
