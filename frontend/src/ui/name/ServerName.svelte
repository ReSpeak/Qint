<script lang="ts">
	import { Connection } from "../../connection";
	import { get } from "svelte/store";
	import { GraphQlServer, Server } from "../../book";

	export let connection: Connection | undefined = undefined;
	export let server: GraphQlServer | Server | undefined = undefined;
	const state = connection?.state;
	const conServer = connection?.book.server;
	const address =
		connection !== undefined ? get(connection.connectOptions).address : (server instanceof GraphQlServer) ? server!.address : undefined;
	$: realServer = conServer !== undefined ? $conServer : server!;
</script>

{#if state !== undefined && !$state.connected}
	<span class="serverName">
		{address}
	</span>
{:else}
	<span class="serverName" style="color:{realServer.color};">
		{realServer.name}
	</span>
{/if}
