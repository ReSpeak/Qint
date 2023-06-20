<script lang="ts">
	import Icon from "../ui/icon/Icon.svelte";
	import { Connection } from "../connection";
	import ClientVolume from "../ui/specialized/ClientVolume.svelte";
	import { app, NodeSelection } from "../app";
	import { Client } from "../book";
	import QuickActionButtons from "../bar/QuickActionButtons.svelte";
	import { Reason } from "../book_events";

	export let connection: Connection;
	export let client: Client;

	const developMode = app.settings.ui._developMode;

	let pokeMessage: string = "";
	let pokeInput: HTMLElement | undefined;

	function onPokeSend() {
		connection.pokeClient(client.id, pokeMessage);
		pokeMessage = "";
	}

	async function kick(reason: Reason) {
		// TODO Handle result
		await connection.sendChange({
			ClientKick: {
				id: client.id,
				reason,
			},
		});
	}

	$: ownClient = client.id === connection.book.ownClientId;
</script>

<div class="inlineButtons">
	<QuickActionButtons selected={new NodeSelection(connection, client)} />
</div>
{#if !ownClient}
	<ClientVolume client={$client} {connection} isInline />
{/if}
{#if $developMode || !ownClient}
	<div class="poke-wrapper">
		<button title="Poke" on:click={onPokeSend}>
			<Icon name="hand-pointing-right" />
		</button>
		<input
			class="input poke-input"
			type="text"
			placeholder="Poke message (optional)"
			bind:this={pokeInput}
			bind:value={pokeMessage}
		/>
	</div>
{/if}
<hr/>
<button on:click={() => kick(Reason.KickChannel)}>
	<Icon name="shoe-formal" />Kick from channel
</button>
<button on:click={() => kick(Reason.KickServer)}>
	<Icon name="shoe-formal" />Kick from server
</button>
<button><Icon name="cancel" />Ban</button>

<style lang="scss">
	.poke-input {
		width: 200px;
		height: 1.8em;
		margin-right: 0.5em;
	}

	.poke-wrapper {
		display: flex;
		align-items: center;
	}

	hr {
		background-color: #656565;
		margin: 0 .5em;
		min-height: 1px;
		height: 1px;
	}
</style>
