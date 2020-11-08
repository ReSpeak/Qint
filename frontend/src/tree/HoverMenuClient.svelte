<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { Connection } from "../connection";
	import type { ClientId } from "../ts";
	import ClientName from "../ui/ClientName.svelte";
	import ClientVolume from "../ui/ClientVolume.svelte";
	import RenderedText from "../ui/RenderedText.svelte";

	export let connection: Connection;
	export let clientId: ClientId;
	let pokeMessage: string = "";
	let pokeInput: HTMLElement | undefined;

	function onPokeSend() {
		connection.sendMessage({
			SendMessage: {
				target: {
					Poke: clientId,
				},
				message: pokeMessage,
			},
		});
		pokeMessage = "";
		// Update chat
		client.chat.update(c => c);
	}

	$: clientRaw = connection.book.clients.get(clientId)!;
	$: client = $clientRaw;
	$: ownClient = clientId === connection.book.ownClientId;
</script>

<div class="name">
	<ClientName {client} />
	{#if client.awayMessage !== null && client.awayMessage.length !== 0}
		({client.awayMessage})
	{/if}
</div>
{#if client.descriptionRendered.length > 0}
<div class="description">
	<RenderedText text={client.descriptionRendered} />
</div>
{/if}
{#if !ownClient}
	<ClientVolume {client} {connection} />
{/if}
<div>
	<button
		class="toolbutton"
		on:click={onPokeSend}
		title="Poke">
		<Icon name="hand-pointing-right"></Icon>
	</button>
	<input class="input poke-input" type="text" placeholder="Poke message (optional)" bind:this={pokeInput} bind:value={pokeMessage}>
</div>

<style lang="scss">
	.poke-input {
		margin: 0 .5em;
		width: 200px;
	}
</style>
