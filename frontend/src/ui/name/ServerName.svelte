<script lang="ts">
	import { Connection } from "../../connection";
	import { get } from "svelte/store";
	import { GraphQlServer, Server } from "../../book";
	import { app } from "../../app";
	import { ServerBase } from "../../bookBase";

	export let connection: Connection | undefined = undefined;
	export let server: ServerBase;

	const state = connection?.state;
	const address =
		connection !== undefined
			? get(connection.connectOptions).address
			: server instanceof GraphQlServer
			? server.address
			: undefined;

	function click() {
		if (server instanceof Server && connection !== undefined) {
			app.select(connection, server);
		}
	}
</script>

{#if state !== undefined && !$state.connected}
	<span>
		{address}
	</span>
{:else}
	<span
		class="nameTag"
		style="color:{server.color};"
		tabindex="0"
		on:click={click}>
		{server.name}
	</span>
{/if}

<style lang="scss">
	@import "./nametag";
</style>
