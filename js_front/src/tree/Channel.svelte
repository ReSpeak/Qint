<script>
	import { afterUpdate } from "svelte";
	import { get } from "svelte/store";
	import Icon from "../ui/Icon.svelte";
	import ChannelIcon from "../ui/ChannelIcon.svelte";
	import FilterString from "../ui/FilterString.svelte";
	import { flash } from "../util";
	import { Channel } from "./book";
	import ClientComp from "./Client.svelte";

	export let connection;
	export let filter;
	export let filterShow = true;
	export let channel;
	let selectedChat = connection.chat.selectedChat;

	let collapsed = false;
	let hovered = false;
	$: children = channel.children;
	$: filterShow = applyFilter($filter, channel, $children);
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

	function applyFilter(filter, channel, children) {
		return filter === "" || channel.name.toLowerCase().includes(filter.toLowerCase())
			|| children.some(c => c.filterShow);
	}

	function switchChannel() {
		connection.switchChannel(channel);
	}

	function setChat() {
		connection.chat.selectChannel(channel);
	}

	function leave(event) {
		if (event.relatedTarget) {
			if (div.contains(event.relatedTarget)) {
				return;
			}
		}
		hovered = false;
	}

	afterUpdate(() => {
		flash(div);
	});
</script>

<li class="container" class:hidden={!filterShow}>
	<div
		bind:this={div}
		class="nameContainer"
		class:ownClient
		class:selectedChannel
		on:mouseover={() => hovered = true} on:mouseout={leave}
	>
		<button class="button collapseButton" on:click={() => collapsed = !collapsed} class:haschildren={$children.length !== 0}>
			<Icon name="chevron-right{collapsed ? '' : ' mdi-rotate-90'}" />
			<ChannelIcon {channel} {connection} />
		</button>
		<button class="button nameButton" on:click={setChat} on:dblclick={switchChannel}>
			<FilterString filter={$filter} content={channel.name} />
		</button>
		{#if hovered}
			<div class="hover menu" style="top: {div.getBoundingClientRect().top}px;">
				<div class="corner"></div>
				{channel.name}
			</div>
		{/if}
	</div>
	<ul class="menu-list" class:collapsed>
		{#each $children as child (child.id)}
			{#if child instanceof Channel}
				<svelte:self {connection} {filter} channel={child} bind:filterShow={child.filterShow} />
			{:else}
				<ClientComp {connection} {filter} client={child} bind:filterShow={child.filterShow} />
			{/if}
		{/each}
	</ul>
</li>

<style lang="scss">
	.container.hidden {
		display: none;
	}

	.button {
		background: none;
		border: none;
		padding: 0.3em;
		height: auto;
		justify-content: start;
	}
	.button:focus {
		box-shadow: none;
	}

	.menu-list li ul {
		margin: 0 0 0 0.5em;
		padding-left: 0.5em;
	}

	.collapseButton {
		display: grid;
		padding: 0;
	}

	.collapseButton > :global(.icon) {
		transition: all 0.1s;
		grid-row: 1;
		grid-column: 1;
		margin: 0;
	}
	.collapseButton > :global(*:first-child) {
		opacity: 0;
	}
	.collapseButton.haschildren:hover > :global(*:first-child) {
		opacity: 1;
	}
	.collapseButton.haschildren:hover > :global(*:last-child) {
		opacity: 0;
	}

	.nameContainer {
		display: grid;
		grid-template-columns: min-content auto;
	}

	.nameButton {
		overflow: hidden;
		padding: 0;
	}

	.nameButton > :global(*) {
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.collapsed {
		display: none;
	}

	.ownClient > :global(*) {
		font-weight: bold;
	}

	.selectedChannel {
		background-color: mix($background, $text, 80%);
	}

	.hover {
		left: calc(var(--channel-tree-width) - 0.5em);
	}
</style>