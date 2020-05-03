<script>
	// TODO Use scroll-anchoring https://blog.eqrion.net/pin-to-bottom/
	import { onMount } from 'svelte';
	import { get } from "svelte/store";
	import Message from "./Message.svelte";
	import Icon from "../ui/Icon.svelte";
	import ClientIcon from "../ui/ClientIcon.svelte";
	import { DateSeparator } from "./chat";
	import LoadableVirtualList from "../ui/LoadableVirtualList.svelte";

	export let connection;
	let chat = connection.chat;

	let messageList;
	let messages;
	let messagesError;
	let messageInput;

	chat.selectedChat.subscribe(_ => {
		if (messageList) {
			messageList.clear();
			messageInput.focus();
		}
	});

	chat.unreadCount.subscribe(_ => {
		if (messageList)
			messageList.newItems(false);
	});

	function sendMessage(e) {
		chat.sendMessage();
		chat.composing = "";
		messageInput.focus();
	}

	async function loadMessages(fromStart) {
		messagesError = undefined;
		try {
			return chat.getMessages(fromStart, messages);
		} catch (err) {
			console.error("Failed to load messages", err);
			messagesError = err;
		}
	}

	function getItemClient(item) {
		if (item.invoker) {
			return {
				uid: item.invoker.uid,
				name: item.invoker.name || item.invokerName,
			};
		} else {
			return { name: item.invokerName };
		}
	}

	onMount(() => {
		messageInput.focus();
	});
</script>

<div class="chat">
	{#if messagesError}
		<div>
			<article class="message is-danger">
				<div class="message-header">
					<p>Error</p>
				</div>
				<div class="message-body">
					Failed to fetch messages
				</div>
			</article>
		</div>
	{:else}
		<LoadableVirtualList bind:this={messageList} bind:items={messages} loadMore={loadMessages} let:item startIsTop={false}>
			<div slot="loading" class="loader"></div>
			{#if item instanceof DateSeparator}
				<div title="{item.date.format('L')}" class="chat-date">
					{item.date.format('LL')}
				</div>
			{:else}
				<div class="invoker-icon">
					<ClientIcon client={item.invoker} {connection} />
				</div>
				<div class="invoker-name has-text-weight-bold">
					<span style={item.getClientColor()}>{item.getClientName()}</span>
				</div>

				{#each item.messages as message}
					<Message {message} />
				{/each}
			{/if}
		</LoadableVirtualList>
	{/if}
	<form class="chat-form" on:submit|preventDefault="{sendMessage}">
		<textarea bind:this={messageInput} bind:value="{chat.composing}" class="input auto_height" name="message"></textarea>
		<button class="button" name="send" type="submit">Send</button>
	</form>
</div>

<style lang="scss">
	.chat {
		display: inline-flex;
		flex-direction: column;
		justify-content: flex-end;
		position: absolute;
		top: 0;
		bottom: 0;
		left: var(--channel-tree-width);
		right: 0;
	}

	.chat-date {
		grid-column-start: 1;
		grid-column-end: 3;
		border-top: 1px solid gray;
		margin: 1em 1em 0em 1em;
		text-align: center;
		color: gray;
	}

	.chat-form {
		display: flex;
		width: 100%;
		padding: 0.5em;

		box-shadow: 0px -5px 20px -5px #bbb;
	}

	.chat-form > * {
		box-sizing: border-box;
		height: 2em;
		font-size: 1em;
	}

	.chat-form textarea {
		flex-grow: 1;
		/*background-color: white;
	color: black;
	border-top: 1px solid black;
	border-right: 1px solid black;
	border-left: none;
	border-bottom: none;*/
	}

	/*.chat-form button {
	background-color: white;
	color: blue;
	vertical-align: middle;
	border-top: 1px solid black;
	border-left: none;
	border-right: none;
	border-bottom: none;
	}*/

	.chat :global(svelte-virtual-list) {
		height: 100%;
	}

	.chat :global(svelte-virtual-list-viewport) {
		height: 100%;
	}

	.chat :global(svelte-virtual-list-contents) {
		padding: 0.5em;
		display: grid;
		grid-template-columns: min-content minmax(0, 1fr);
	}

	.invoker-icon {
		padding: 0.1em 0.5em;
	}

	.invoker-icon {
		grid-column: 1;
		display: flex;
		margin-top: 0.7em;
	}

	.invoker-name {
		grid-column: 2;
		margin-top: 0.7em;
	}
</style>
