<script>
	import { afterUpdate } from "svelte";
	import ClientIcon from "../ui/ClientIcon.svelte";
	import Icon from "../ui/Icon.svelte";
	import { flash } from "../util";

	export let connection;
	export let client;
	let selectedChat = connection.chat.selectedChat;
	let hovered = false;
	let volume = 1.0;

	$: ownClient = client.id === connection.ownClient;
	$: selectedClient = "Client" in $selectedChat && $selectedChat.Client === client.id;
	$: loadVolume(hovered);
	let div;
	let volumeUpdated;

	function setChat() {
		connection.chat.selectClient(client);
	}

	function leave(event) {
		if (event.relatedTarget) {
			if (div.contains(event.relatedTarget)) {
				return;
			}
		}
		hovered = false;
	}

	async function loadVolume(hovered) {
		if (hovered) {
			volumeUpdated = false;
			await client.loadVolume();
			if (!volumeUpdated)
				volume = client.volume;
		}
	}

	function toggleVolume() {
		if (volume == 0.0) {
			volume = 1.0;
		} else {
			volume = 0.0;
		}
		updateVolume();
	}

	function updateVolume() {
		volumeUpdated = true;
		// TODO This should not be linear
		client.updateVolume(connection, volume);
	}

	afterUpdate(() => {
		flash(div);
	});
</script>

<li>
	<div
		bind:this={div}
		class:ownClient
		class:selectedClient
		on:mouseover={() => hovered = true} on:mouseout={leave}
	>
		<button class="button clientButton" on:click={setChat}>
			<ClientIcon {client} {connection} />
			<span style={client.getColor()}>{client.name}</span>
		</button>
		{#if hovered}
			<div class="hover menu" style="top: {div.getBoundingClientRect().top}px;">
				<div class="corner"></div>
				<div class="name" style={client.getColor()}>{client.name}</div>
				<button class="volume button" on:click={toggleVolume}>
					{#if volume == 0.0}
						<Icon name="volume-off" />
					{:else}
						<Icon name="volume-high" />
					{/if}
				</button>
				<input type="range" min="0" max="2" step="0.01" bind:value={volume}
					class="volume slider" title="Volume: {volume}" on:change={updateVolume} />
			</div>
		{/if}
	</div>
</li>

<style>
	.clientButton {
		background: none;
		border: none;
		padding: 0.2em 1em 0.2em 1em;
		height: auto;
		width: 100%;
		justify-content: start;
	}
	.clientButton:focus {
		box-shadow: none;
	}

	.ownClient :global(span) {
		font-weight: bold;
	}

	.selectedClient {
		background-color: #ddd;
	}

	.hover {
		left: calc(var(--channel-tree-width) - 0.5em);
		display: grid;
		grid-gap: 1em;
	}

	.hover .name {
		grid-row: 1;
		grid-column: 1 / 3;
	}

	.hover .volume.button {
		grid-row: 2;
		grid-column: 1;
	}

	.hover .volume.slider {
		grid-row: 2;
		grid-column: 2;
	}
</style>