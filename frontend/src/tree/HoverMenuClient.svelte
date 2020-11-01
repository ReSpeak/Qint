<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { Connection } from "../connection";
	import type { ClientId } from "../ts";
	import ClientName from "../ui/ClientName.svelte";
	import ClientVolume from "../ui/ClientVolume.svelte";
	import RenderedText from "../ui/RenderedText.svelte";
	import BModal from "../ui/BModal.svelte";
	import { tick } from "svelte";

	export let connection: Connection;
	export let clientId: ClientId;
	let pokeModalVisible = false;
	let pokeMessage: string;
	let pokeInput: HTMLElement | undefined;

	async function onPokeClick() {
		pokeModalVisible = true;
		await tick();
		if (pokeInput !== undefined)
			pokeInput.focus();
	}

	function onPokeSend() {
		connection.sendMessage({
			SendMessage: {
				target: {
					Poke: clientId,
				},
				message: pokeMessage,
			},
		});
		pokeModalVisible = false;
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
		on:click={onPokeClick}
		title="Poke">
		<Icon name="hand-pointing-right"></Icon>
	</button>
</div>
<form on:submit|preventDefault={onPokeSend}>
	<BModal bind:visible={pokeModalVisible}>
		<div slot="header">
			Poke <ClientName {client} />
		</div>
		<input class="input" type="text" bind:this={pokeInput} bind:value={pokeMessage}>
		<button type="submit" slot="footer" class="button is-success">Poke</button>
	</BModal>
</form>

<style lang="scss">
</style>
