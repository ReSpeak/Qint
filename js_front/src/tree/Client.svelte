<script>
	import { afterUpdate } from "svelte";
	import ClientIcon from "../ui/ClientIcon.svelte";
	import FilterString from "../ui/FilterString.svelte";
	import Icon from "../ui/Icon.svelte";

	export let connection;
	export let filter;
	export let filterShow = true;
	export let client;
	let selectedChat = connection.chat.selectedChat;
	let hovered = false;
	let newHover = false;
	// Volume is in dB, https://www.dr-lex.be/info-stuff/volumecontrols.html
	let minVolume = -30;
	let maxVolume = +30;
	let volume = 0;

	$: filterShow = applyFilter($filter, client);
	$: ownClient = client.id === connection.ownClientId;
	$: selectedClient = "Client" in $selectedChat && $selectedChat.Client === client.id;
	$: loadVolume(hovered);
	let div;
	let volumeUpdated;
	let volumeTimer;

	function setChat() {
		connection.chat.selectClient(client);
	}

	function applyFilter(filter, client) {
		return filter === "" || client.name.toLowerCase().includes(filter.toLowerCase());
	}

	function hover() {
		hovered = true;
		newHover = true;
	}

	function leave(event) {
		if (event.relatedTarget) {
			if (div.contains(event.relatedTarget)) {
				return;
			}
		}
		newHover = false;
		setTimeout(() => {
			if (!newHover)
				hovered = false;
		}, 50);
	}

	async function loadVolume(hovered) {
		if (hovered) {
			volumeUpdated = false;
			await client.loadVolume();
			if (!volumeUpdated) {
				if (client.volume === 0) {
					volume = minVolume;
				} else {
					volume = Math.round(20 * Math.log10(client.volume));
				}
			}
		}
	}

	function toggleVolume() {
		if (volume === minVolume) {
			volume = 0;
		} else {
			volume = minVolume;
		}
		updateVolume();
	}

	function updateVolume() {
		volumeUpdated = true;
		if (volumeTimer)
			return;
		// Update every few ms
		volumeTimer = setTimeout(() => {
			volumeTimer = undefined;
			let vol = 0;
			if (volume !== minVolume) {
				vol = Math.pow(10, volume / 20);
			}
			client.updateVolume(connection, vol);
		}, 100);
	}
</script>

<li class="container" class:hidden={!filterShow}>
	<div
		bind:this={div}
		class:ownClient
		class:selectedClient
		on:mouseover={hover} on:mouseout={leave}
		on:focusin={hover} on:focusout={leave}
	>
		<button class="button clientButton" class:talking={client.talking !== undefined} on:click={setChat}>
			<div class="inner"></div>
			<ClientIcon {client} {connection} />
			<span style={client.getColor()}><FilterString filter={$filter} content={client.name} /></span>
			<span class="icons">
				{#if client.input_muted}
					<Icon name="microphone-off" style="color: red;" />
				{/if}
				{#if client.output_muted}
					<Icon name="volume-off" style="color: red;" />
				{/if}
				{#if client.away_message !== null}
					<Icon name="sleep" style="color: rgb(70,180,255);" />
				{/if}
			</span>
		</button>
		{#if hovered}
			<div class="hover menu" style="top: {div.getBoundingClientRect().top}px;">
				<div class="corner"></div>
				<div class="name">
					<span style={client.getColor()}>{client.name}</span>
					{#if client.away_message !== null && client.away_message.length !== 0}
						({client.away_message})
					{/if}
				</div>
				<button class="volume button" on:click={toggleVolume}>
					{#if volume === minVolume}
						<Icon name="volume-off" />
					{:else}
						<Icon name="volume-high" />
					{/if}
				</button>
				<input type="range" min={minVolume} max={maxVolume} step="2" bind:value={volume}
					class="volume slider" title="{volume} dB" on:input={updateVolume} />
			</div>
		{/if}
	</div>
</li>

<style lang="scss">
	.container.hidden {
		display: none;
	}

	.clientButton {
		background: none;
		border: none;
		padding: 0;
		padding-left: 0.5em;
		height: auto;
		width: 100%;
		overflow: hidden;
		justify-content: start;
		position: relative;
		z-index: 1;

		display: grid;
		grid-auto-flow: column;
		grid-template-columns: min-content min-content 1fr;
	}
	.clientButton:focus {
		box-shadow: none;
	}

	.clientButton > :global(*) {
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.ownClient :global(span) {
		font-weight: bold;
	}

	.selectedClient {
		background-color: mix($background, $text, 80%);
	}

	.icons {
		display: flex;
		flex-wrap: nowrap;
		justify-self: end;
	}

	.button .icons > :global(span) {
		margin: 0;
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

	.clientButton .inner {
		transition: opacity 0.2s ease-in-out, height 0.2s ease-in-out;
		position: absolute;
		// top: 0;
		right: 0;
		bottom: 0;
		left: 0;
		z-index: -1;

		background-image: url("/talking.svg");
		background-size: 100% auto;
		-webkit-mask-image: radial-gradient(rgba(0,0,0,1), rgba(0,0,0,0));
		mask-image: radial-gradient(rgba(0,0,0,1), rgba(0,0,0,0));

		opacity: 0;
		height: 50%;
	}

	.clientButton.talking .inner {
		opacity: 1;
		height: 100%;
	}
</style>
