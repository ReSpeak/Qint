<svelte:options immutable />

<script lang="ts">
	import { Channel, Client, Server, ServerGroup } from "../book";
	import { Connection } from "../connection";
	import { TsNotification } from "../notification";
	import ClientName from "../ui/ClientName.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import type { Invoker } from "../backend/ws";

	export let connection: Connection;
	export let notification: TsNotification;

	const args = notification.args;

	// Make the svelte typechecker happy
	function toClient(a: any): Client {
		return a as Client;
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

{#each notification.pieces as piece, i}
	{piece}
	{#if i < args.length}
		{#if args[i] instanceof Client}
			<ClientName client={toClient(args[i])} />
		{:else if args[i] instanceof Server}
			<ServerName {connection} />
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

<style lang="scss">
	.unknown {
		background-color: red;
	}
</style>
