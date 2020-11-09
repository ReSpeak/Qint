<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { Connection } from "../connection";
	import type { ClientId } from "../ts";
	import ClientName from "../ui/ClientName.svelte";
	import ClientVolume from "../ui/ClientVolume.svelte";
	import RenderedText from "../ui/RenderedText.svelte";
	import { app } from "../app";
	import { Client } from "../book";

	export let connection: Connection;
	export let client: Client;
	let pokeMessage: string = "";
	let pokeInput: HTMLElement | undefined;
	let developMode = app.transientSettings.ui._developMode;

	function onPokeSend() {
		connection.sendMessage({
			SendMessage: {
				target: {
					Poke: client.id,
				},
				message: pokeMessage,
			},
		});
		pokeMessage = "";
		// Update chat
		client.chat.update(c => c);
	}

	$: ownClient = client.id === connection.book.ownClientId;
</script>

<div class="name">
	<ClientName client={$client} />
	{#if $client.awayMessage !== null && $client.awayMessage.length !== 0}
		({$client.awayMessage})
	{/if}
</div>
{#if $client.descriptionRendered.length > 0}
<div class="description">
	<RenderedText text={$client.descriptionRendered} />
</div>
{/if}
{#if !ownClient}
	<ClientVolume client={$client} {connection} />
{/if}
{#if $developMode || !ownClient}
	<div>
		<button
			class="toolbutton"
			on:click={onPokeSend}
			title="Poke">
			<Icon name="hand-pointing-right"></Icon>
		</button>
		<input class="input poke-input" type="text" placeholder="Poke message (optional)" bind:this={pokeInput} bind:value={pokeMessage}>
	</div>
{/if}

<style lang="scss">
	.poke-input {
		margin: 0 .5em;
		width: 200px;
	}
</style>
