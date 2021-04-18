<script lang="ts">
	import Icon from "../ui/Icon.svelte";
	import { Connection } from "../connection";
	import ClientName from "../ui/ClientName.svelte";
	import ClientVolume from "../ui/ClientVolume.svelte";
	import { app } from "../app";
	import { Client } from "../book";

	export let connection: Connection;
	export let client: Client;
	let pokeMessage: string = "";
	let pokeInput: HTMLElement | undefined;
	const developMode = app.transientSettings.ui._developMode;

	function onPokeSend() {
		connection.pokeClient(client.id, pokeMessage);
		pokeMessage = "";
	}

	$: ownClient = client.id === connection.book.ownClientId;
</script>

<div class="name">
	<ClientName client={$client} />
	{#if $client.awayMessage !== null && $client.awayMessage.length !== 0}
		({$client.awayMessage})
	{/if}
</div>
{#if $client.description.length > 0}
<div class="description">
	{$client.description}
</div>
{/if}
{#if !ownClient}
	<ClientVolume client={$client} {connection} />
{/if}
{#if $developMode || !ownClient}
	<form on:submit|preventDefault={onPokeSend}>
		<button type="submit" class="toolbutton" title="Poke">
			<Icon name="hand-pointing-right" />
		</button>
		<input
			class="input poke-input"
			type="text"
			placeholder="Poke message (optional)"
			bind:this={pokeInput}
			bind:value={pokeMessage} />
	</form>
{/if}

<style lang="scss">
	.poke-input {
		margin: 0 0.5em;
		width: 200px;
	}
</style>
