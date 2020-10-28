<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { Connection } from "../connection";
	import type { ClientId } from "../ts";
	import ClientName from "../ui/ClientName.svelte";
	import ClientVolume from "../ui/ClientVolume.svelte";
	import RenderedText from "../ui/RenderedText.svelte";
	import BModal from "../ui/BModal.svelte";

	export let connection: Connection;
	export let clientId: ClientId;
	let pokeModalVisible = false;

	function onPokeClick(e: MouseEvent) {
		pokeModalVisible = true;
	}

	function onPokeSend(e: MouseEvent) {
		console.log("yeet");
		pokeModalVisible = false;
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
<BModal title={"Poke " + client.name} bind:visible={pokeModalVisible}>
	<div slot="content">
		<input class="input" type="text">
	</div>
	<button slot="footer" on:click={onPokeSend} class="button is-success">Poke</button>
</BModal>

<style lang="scss">
</style>
