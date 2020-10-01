<script lang="typescript">
	import { Connection } from "../connection";

	export let connection: Connection;
	const state = connection.state;
	const server = connection.book.server;
</script>

{#if !$state.connected}
	<span>
		{connection.connectOptions.Connect.address}
	</span>
{:else}
	<span style={$server.getColor()}>
		{$server.name}
	</span>
{/if}
