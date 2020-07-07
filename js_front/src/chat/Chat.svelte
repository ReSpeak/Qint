<script lang="typescript">
	// TODO Use scroll-anchoring https://blog.eqrion.net/pin-to-bottom/
	import { onMount } from 'svelte';
	import { get } from "svelte/store";
	import Message from "./Message.svelte";
	import Icon from "../ui/Icon.svelte";
	import ClientIcon from "../ui/ClientIcon.svelte";
	import * as i_chat from "./chat";
	import LoadableVirtualList from "../ui/LoadableVirtualList.svelte";
	import LazyList from "../ui/LazyList.svelte";
	import { ListFetchDir, ILazyList } from "../ui/lazyList";
	import { Connection } from "../connection";
	import { assert } from "../util";

	export let connection: Connection = undefined as any;
	assert(fetchElements, "No connection provided");
	let chat = connection.chat;

	let chatList: ILazyList;
	let messagesError: unknown | undefined;
	let messageInput: HTMLTextAreaElement;

	chat.selectedChat.subscribe(_ => {
		console.log("switch chat");
		if (chatList) {
			chatList.clear();
			chatList.sourceChanged(ListFetchDir.New);
		}
		if (messageInput)
			messageInput.focus();
	});

	chat.unreadCount.subscribe(_ => {
		if (chatList)
			chatList.sourceChanged(ListFetchDir.After);
	});

	function sendMessage(e: Event) {
		chat.sendMessage();
		chat.composing = "";
		messageInput.focus();
	}

	async function fetchElements(idFrom: i_chat.GroupedMessages | undefined, dir: ListFetchDir) {
		messagesError = undefined;
		try {
			console.log("fetching data");
			return chat.getMessages(idFrom, dir);
		} catch (err) {
			console.error("Failed to load messages", err);
			messagesError = err;
			return i_chat.Chat.EmptyFetch;
		}
	}

	function getItemClient(item: i_chat.Message) {
		if (item.invoker) {
			return {
				uid: item.invoker.uid,
				name: item.invoker.name ?? item.invokerName,
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
		<LazyList bind:this={chatList} {fetchElements} let:item>
			<div slot="loading" class="loader"></div>
			{#if item.displayDateSeparator}
				<div title="{item.topDate.format('L')}" class="chat-date">
					{item.topDate.format('LL')}
				</div>
			{/if}
			<div class="invoker-icon">
				<ClientIcon client={item.invoker} {connection} />
			</div>
			<div class="invoker-name has-text-weight-bold">
				<span style={item.clientColor}>{item.displayName}</span>
			</div>

			{#each item.messages as message}
				<Message {message} />
			{/each}
		</LazyList>
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
		line-height: 1.2;
	}

	.chat-date {
		grid-column-start: 1;
		grid-column-end: 3;
		border-top: 1px solid mix($text, $background, 60%);
		margin: 0.2em 1em 0em 1em;
		text-align: center;
		color: mix($text, $background, 60%);
	}

	.chat-form {
		display: flex;
		width: 100%;

		box-shadow: 0px -5px 20px -5px #0005;
	}

	.chat-form > * {
		box-sizing: border-box;
		height: 2em;
		font-size: 1em;
	}

	.chat-form textarea {
		flex-grow: 1;
		border-top: 1px solid;
		border-right: 1px solid;
		border-left: none;
		border-bottom: none;
		border-radius: 0;
	}

	.chat-form button {
		background: none;
		color: $blue;
		vertical-align: middle;
		border-top: 1px solid;
		border-left: none;
		border-right: none;
		border-bottom: none;
		border-radius: 0;
	}

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

	.chat :global(.scrollPane) {
		padding: 0.5em;
		display: grid;
		grid-template-columns: min-content minmax(0, 1fr);
	}

	.invoker-icon {
		grid-column: 1;
		display: flex;
		margin-top: 0.5em;
		padding: 0 0.5em;
	}

	.invoker-name {
		grid-column: 2;
		margin-top: 0.5em;
	}
</style>
