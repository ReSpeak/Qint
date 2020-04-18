<script>
	import { afterUpdate } from "svelte";
	import { get } from "svelte/store";
	import Icon from "../ui/Icon.svelte";
	import ChannelIcon from "../ui/ChannelIcon.svelte";
	import { flash } from "../util";
	import { Channel } from "./book";
	import ClientComp from "./Client.svelte";

	export let connection;
	export let channel;
	let children = channel.children;
	let selectedChat = connection.chat.selectedChat;

	let collapsed = false;
	// Update if a client moves in or out
	$: ownClient = updateOwnClient($children);
	$: selectedChannel = "Channel" in $selectedChat && $selectedChat.Channel === channel.id;
	let div;

	function updateOwnClient() {
		let isOwn = false;
		let client = get(connection.book.clients).get(connection.ownClient);
		if (client) {
			isOwn = client.channel === channel.id;
		}
		return isOwn;
	}

	function switchChannel() {
		connection.switchChannel(channel);
	}

	function setChat() {
		connection.chat.selectChannel(channel);
	}

	afterUpdate(() => {
		flash(div);
	});
</script>

<li>
	<div
		bind:this={div}
		class="nameContainer"
		class:ownClient
		class:selectedChannel
	>
		<button class="button" on:click={() => collapsed = !collapsed} class:invisible={$children.length == 0}>
			<Icon name="chevron-right{collapsed ? '' : ' mdi-rotate-90'}" />
		</button>
		<button class="button nameButton" on:click={setChat} on:dblclick={switchChannel}>
			<ChannelIcon {channel} {connection} />
			<span>{channel.name}</span>
		</button>
	</div>
	<ul class="menu-list" class:collapsed>
		{#each $children as child}
			{#if child instanceof Channel}
				<svelte:self {connection} channel={child} />
			{:else}
				<ClientComp {connection} client={child} />
			{/if}
		{/each}
	</ul>
</li>

<style>
	.button {
		background: none;
		border: none;
		padding: 0.3em;
		height: auto;
		width: 100%;
		justify-content: start;
	}
	.button:focus {
		box-shadow: none;
	}

	.menu-list li ul {
		margin: 0 0 0 0.5em;
		padding-left: 0.5em;
	}

	.nameContainer {
		display: grid;
		grid-template-columns: min-content auto;
	}

	.nameButton :global(.icon) {
		margin-left: 0;
	}

	.invisible {
		visibility: hidden;
	}

	.collapsed {
		display: none;
	}

	.ownClient span {
		font-weight: bold;
	}

	.selectedChannel {
		background-color: #ddd;
	}
</style>