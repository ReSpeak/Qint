<script lang="typescript">
	import { Connection } from "../connection";

	export let connection: Connection;
	const state = connection.state;
	const server = connection.book.server;
</script>

{#if !$state.connected}
	<span class="serverName">
		{connection.connectOptions.address}
	</span>
{:else}
	<span class="serverName" style={$server.color}>
		{$server.name}
	</span>
{/if}
