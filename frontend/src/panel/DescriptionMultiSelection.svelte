<script lang="ts">
	import { Connection } from "../connection";
	import { NodeSelections } from "../app";
	import ServerName from "../ui/name/ServerName.svelte";
	import { Channel, Client, Server } from "../book";
	import ClientName from "../ui/name/ClientName.svelte";

	export let selected: NodeSelections;

	let channelCount = 0;
	let clientCount = 0;
	let serverCount = 0;
	let connectionCount = 0;
	let connection: Connection | undefined;

	$: update(selected);

	function update(selected: NodeSelections) {
		channelCount = 0;
		clientCount = 0;
		serverCount = 0;
		connectionCount = 0;
		const cons = new Set();
		for (const sel of selected.selections) {
			if (sel.node.qlType === "SERVER") {
				serverCount++;
			} else {
				if (sel.node.qlType === "CHANNEL") channelCount++;
				else if (sel.node.qlType === "CLIENT" || sel.node.qlType === "POKE") clientCount++;

				if (!cons.has(sel.connection)) {
					cons.add(sel.connection);
					connectionCount++;
				}
			}
		}
		connection = selected.getConnection();
	}
</script>

<h5 class="title is-5">
	Selected
	{#if clientCount > 0 || channelCount > 0}
		{#if clientCount > 0}
			{clientCount} client{#if clientCount > 1}s{/if}
		{/if}
		{#if channelCount > 0}
			{#if clientCount > 0}
				and
			{/if}
			{channelCount} channel{#if channelCount > 1}s{/if}
		{/if}
		on
		{#if connection !== undefined}
			<ServerName {connection} server={connection.book.server} />
		{:else}
			{connectionCount} servers
		{/if}
	{/if}
	{#if serverCount > 0}
		{#if clientCount > 0 || channelCount > 0}
			and
		{/if}
		{serverCount} server{#if serverCount > 1}s{/if}
	{/if}
</h5>
<ul>
	{#each selected.selections as sel}
		<li>
			{#if sel.node instanceof Channel}
				{sel.node.name} on <ServerName
					connection={sel.connection}
					server={sel.connection.book.server} />
			{:else if sel.node instanceof Client}
				<ClientName connection={sel.connection} client={sel.node} /> on <ServerName
					connection={sel.connection}
					server={sel.connection.book.server} />
			{:else if sel.node instanceof Server}
				<ServerName connection={sel.connection} server={sel.node} />
			{:else}
				{sel.node.qlId}
			{/if}
		</li>
	{/each}
</ul>

<style lang="scss">
</style>
