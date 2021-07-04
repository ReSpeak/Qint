<svelte:options immutable />

<script lang="ts">
	import { Channel, Client, Server, ServerGroup } from "../book";
	import { Connection } from "../connection";
	import { TsNotification } from "../notifications";
	import ClientName from "../ui/name/ClientName.svelte";
	import ServerName from "../ui/name/ServerName.svelte";
	import type { Invoker } from "../backend/ws";

	export let connection: Connection;
	export let notification: TsNotification;

	const args = notification.args;

	// Make the svelte typechecker happy
	function toClient(a: any): Client {
		return a as Client;
	}
	function toServer(a: any): Server {
		return a as Server;
	}
	function getName(a: any): string {
		return a.name;
	}
	function hasName(a: any): a is Invoker {
		return a !== null && a !== undefined && "name" in a;
	}
	function getClientFromInvoker(i: any): Client | undefined {
		return connection.book.getClient(i.id.toString());
	}
</script>

<h6 class="title is-6">
	<ServerName server={connection.book.server} {connection} />
</h6>
<div class="content">
	{#each notification.pieces as piece, i}
		{piece}
		{#if i < args.length}
			{#if args[i] instanceof Client}
				<ClientName client={toClient(args[i])} {connection} />
			{:else if args[i] instanceof Server}
				<ServerName server={toServer(args[i])} {connection} />
			{:else if args[i] instanceof Channel}
				<span class="channel">{getName(args[i])}</span>
			{:else if args[i] instanceof ServerGroup}
				<span class="serverGroup">{getName(args[i])}</span>
			{:else if typeof args[i] === "string" || args[i] instanceof String}
				{args[i]}
			{:else if hasName(args[i])}
				<!-- Invoker -->
				{#if getClientFromInvoker(args[i]) !== undefined}
					<ClientName client={toClient(getClientFromInvoker(args[i]))} />
				{:else}
					{getName(args[i])}
				{/if}
			{:else}
				<span class="unknown">{args[i]}</span>
			{/if}
		{/if}
	{/each}
</div>

<style lang="scss">
	.title.is-6 {
		font-size: 0.8em;
		margin-bottom: 0.1em;
	}

	.unknown {
		background-color: red;
	}
</style>
