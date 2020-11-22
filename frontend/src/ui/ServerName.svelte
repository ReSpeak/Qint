<script lang="typescript">
	import { Connection } from "../connection";
	import { get } from "svelte/store";

	export let connection: Connection;
	const state = connection.state;
	const server = connection.book.server;
	const address = get(connection.connectOptions).address;
</script>

{#if !$state.connected}
	<span class="serverName">
		{address}
	</span>
{:else}
	<span class="serverName" style={$server.color}>
		{$server.name}
	</span>
{/if}
