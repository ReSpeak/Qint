<script lang="ts">
	import { Connection } from "../../connection";
	import { get } from "svelte/store";
	import { GraphQlServer, Server } from "../../book";
	import { app } from "../app";

	export let connection: Connection | undefined = undefined;
	export let server: GraphQlServer | Server | undefined = undefined;

	const state = connection?.state;
	const conServer = connection?.book.server;
	const address =
		connection !== undefined ? get(connection.connectOptions).address : (server instanceof GraphQlServer) ? server!.address : undefined;
	$: realServer = conServer !== undefined ? $conServer : server!;

	function click() {
		app.select(connection, conServer ?? server!);
	}
</script>

{#if state !== undefined && !$state.connected}
	<span class="serverName">
		{address}
	</span>
{:else}
	<span class="serverName button noBut" style="color:{realServer.color};" tabindex="0" on:click={click}>
		{realServer.name}
	</span>
{/if}