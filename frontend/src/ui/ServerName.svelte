<script lang="typescript">
	import { Connection } from "../connection";

	export let connection: Connection;
	const state = connection.state;
	const server = connection.book.server;
</script>

{#if !$state.connected}
	<span>
		{connection.connectOptions.address}
	</span>
{:else}
	<span style={$server.color}>
		{$server.name}
	</span>
{/if}
