<svelte:options immutable />

<script lang="ts">
	import { Book, Channel, Client, Server, ServerGroup } from "../book";
	import { Connection } from "../connection";
	import { TsNotification } from "../notifications";
	import ClientName from "../ui/name/ClientName.svelte";
	import ServerName from "../ui/name/ServerName.svelte";
	import type { Invoker } from "../backend/ws";
	import { LONG_DATETIME } from "../util";

	export let connection: Connection;
	export let notification: TsNotification;

	const args = notification.args;

	function hasName(a: Invoker | Book): a is Invoker {
		return "name" in a;
	}
	function getClientFromInvoker(i: Invoker): Client | undefined {
		return connection.book.getClient(i.id.toString());
	}
</script>

<h6 class="title is-6">
	<span class="date" title={notification.date.format(LONG_DATETIME)}>
		{notification.date.format("HH:mm")}
	</span>
	<ServerName server={connection.book.server} />
</h6>
<div class="content">
	{#each notification.pieces as piece, i}
		{@const arg = args[i]}
		{piece}
		{#if arg != null}
			{#if arg instanceof Client}
				<ClientName client={arg} {connection} />
			{:else if arg instanceof Server}
				<ServerName server={arg} />
			{:else if arg instanceof Channel}
				<span class="channel">{arg.name}</span>
			{:else if arg instanceof ServerGroup}
				<span class="serverGroup">{arg.name}</span>
			{:else if typeof arg === "string" || arg instanceof String}
				{arg}
			{:else if hasName(arg)}
				<!-- Invoker -->
				{#if getClientFromInvoker(arg) !== undefined}
					<ClientName client={getClientFromInvoker(arg)!} />
				{:else}
					{arg.name}
				{/if}
			{:else}
				<span class="unknown">{arg}</span>
			{/if}
		{/if}
	{/each}
</div>

<style lang="scss">
	.title.is-6 {
		font-size: 0.8em;
		margin-bottom: 0.1em;
	}

	.date {
		font-weight: normal;
	}

	.unknown {
		background-color: red;
	}
</style>
