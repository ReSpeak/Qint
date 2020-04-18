<script>
	import { afterUpdate } from "svelte";
	import Icon from "../ui/Icon.svelte";
	import ChannelIcon from "../ui/ChannelIcon.svelte";
	import { flash } from "../util";
	import { Channel } from "./book";
	import ClientComp from "./Client.svelte";

	export let connection;
	export let channel;
	let children = channel.children;

	let collapsed = false;
	// TODO dummy
	let ownClient = false;
	let selectedChannel = false;
	let div;

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
		class="flex-line"
		class:own-client="{ownClient}"
		class:selected-channel="{selectedChannel}"
	>
		<button class="button collapse-button" on:click={() => collapsed = !collapsed}>
			<Icon name="chevron-right{collapsed ? '' : ' mdi-rotate-90'}" />
		</button>
		<button class="button expand" on:click={setChat} on:dblclick={switchChannel}>
			<ChannelIcon {channel} {connection} />
			<span class="expand">{channel.name}</span>
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
	}
	.button:focus {
		box-shadow: none;
	}

	.menu-list li ul {
		margin: 0 0 0 0.5em;
		padding-left: 0.5em;
	}

	.collapsed {
		display: none;
	}
</style>