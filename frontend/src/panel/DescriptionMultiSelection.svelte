<script lang="ts">
	import { Connection } from "../connection";
	import { NodeSelections } from "../app";
	import ServerName from "../ui/name/ServerName.svelte";
	import { Channel, Client, Server } from "../book";
	import ClientName from "../ui/name/ClientName.svelte";
	import Icon from "../ui/icon/Icon.svelte";
	import { Reason } from "../book_events";
	import type { ChannelId, ClientId } from "../ts";

	export let selected: NodeSelections;

	let channelCount = 0;
	let clientCount = 0;
	let serverCount = 0;
	let connectionCount = 0;
	let allClientsMuted = true;
	let isClientWhispering = false;
	let isChannelWhispering = false;
	let connection: Connection | undefined;

	$: update(selected);

	function update(selected: NodeSelections) {
		channelCount = 0;
		clientCount = 0;
		serverCount = 0;
		connectionCount = 0;
		allClientsMuted = true;
		isClientWhispering = false;
		isChannelWhispering = false;
		const cons = new Set();
		for (const sel of selected.selections) {
			if (sel.node.qlType === "SERVER") {
				serverCount++;
			} else {
				if (sel.node.qlType === "CHANNEL") {
					channelCount++;
					isChannelWhispering ||= sel.connection.isWhispering;
				} else if (sel.node.qlType === "CLIENT" || sel.node.qlType === "POKE") {
					clientCount++;
					const client = sel.node as Client;
					client.loadVolume().then(() => (allClientsMuted &&= client.volume === 0));
					isClientWhispering ||= sel.connection.isWhispering;
				}

				if (!cons.has(sel.connection)) {
					cons.add(sel.connection);
					connectionCount++;
				}
			}
		}
		connection = selected.getConnection();
	}

	async function kickClients(reason: Reason) {
		// TODO Handle result
		for (const sel of selected.selections) {
			if (sel.node.qlType === "CLIENT") {
				await sel.connection.sendChange({
					ClientKick: {
						id: (sel.node as Client).id,
						reason,
					},
				});
			}
		}
	}

	async function muteClients() {
		// TODO Handle result
		for (const sel of selected.selections) {
			if (sel.node.qlType === "CLIENT") {
				const client = sel.node as Client;
				client.updateVolume(sel.connection, allClientsMuted ? client.prevVolume ?? 1 : 0);
			}
		}
		update(selected);
	}

	async function whisperClients() {
		if (isClientWhispering) {
			for (const sel of selected.selections) {
				if (sel.node.qlType === "CLIENT" && sel.connection.isWhispering)
					sel.connection.stopWhispering();
			}
		} else {
			const targets: Map<Connection, ClientId[]> = new Map();
			for (const sel of selected.selections) {
				if (sel.node.qlType === "CLIENT") {
					if (!targets.has(sel.connection)) targets.set(sel.connection, []);
					targets.get(sel.connection)!.push((sel.node as Client).id);
				}
			}

			for (const [con, ids] of targets) con.startWhispering(ids, []);
		}
		update(selected);
	}

	async function whisperChannels() {
		if (isChannelWhispering) {
			for (const sel of selected.selections) {
				if (sel.node.qlType === "CHANNEL" && sel.connection.isWhispering)
					sel.connection.stopWhispering();
			}
		} else {
			const targets: Map<Connection, ChannelId[]> = new Map();
			for (const sel of selected.selections) {
				if (sel.node.qlType === "CHANNEL") {
					if (!targets.has(sel.connection)) targets.set(sel.connection, []);
					targets.get(sel.connection)!.push((sel.node as Channel).id);
				}
			}

			for (const [con, ids] of targets) con.startWhispering([], ids);
		}
		update(selected);
	}

	async function disconnectServers() {
		for (const sel of selected.selections) {
			if (sel.node.qlType === "SERVER") {
				sel.connection.disconnect();
			}
		}
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
					server={sel.connection.book.server}
				/>
			{:else if sel.node instanceof Client}
				<ClientName connection={sel.connection} client={sel.node} /> on <ServerName
					connection={sel.connection}
					server={sel.connection.book.server}
				/>
			{:else if sel.node instanceof Server}
				<ServerName connection={sel.connection} server={sel.node} />
			{:else}
				{sel.node.qlId}
			{/if}
		</li>
	{/each}
</ul>

{#if clientCount > 0}
	<div class="descGroup">
		<p class="buttons">
			<button class="button is-small is-info" on:click={whisperClients}>
				<Icon name="microphone" />
				<span class:is-loading={isClientWhispering} />
				<span>
					{#if !isClientWhispering}
						Whisper to selected clients
					{:else}
						Stop whispering
					{/if}
				</span>
			</button>
			<button class="button is-small is-info" on:click={muteClients}>
				<Icon name={allClientsMuted ? "microphone-off" : "microphone"} />
				<span
					>{#if allClientsMuted}Unmute{:else}Mute{/if}</span
				>
			</button>
			<button
				class="button is-small is-warning"
				on:click={() => kickClients(Reason.KickChannel)}
			>
				<Icon name="shoe-formal" />
				<span>Kick Channel</span>
			</button>
			<button
				class="button is-small is-danger"
				on:click={() => kickClients(Reason.KickServer)}
			>
				<Icon name="shoe-formal" />
				<span>Kick Server</span>
			</button>
			<button class="button is-small is-danger">
				<Icon name="cancel" />
				<span>Ban</span>
			</button>
		</p>
	</div>
{/if}

{#if channelCount > 0}
	<div class="descGroup">
		<p class="buttons">
			<button class="button is-small is-info" on:click={whisperChannels}>
				<Icon name="microphone" />
				<span class:is-loading={isChannelWhispering} />
				<span>
					{#if !isChannelWhispering}
						Whisper to selected channels
					{:else}
						Stop whispering
					{/if}
				</span>
			</button>
		</p>
	</div>
{/if}

{#if serverCount > 0}
	<div class="descGroup">
		<p class="buttons">
			<button class="button is-small is-warning" on:click={disconnectServers}>
				<Icon name="shoe-formal" />
				<span>Disconnect</span>
			</button>
		</p>
	</div>
{/if}

<style lang="scss">
	span.is-loading {
		@include loader;
		margin-right: 0.5em;
	}
</style>
