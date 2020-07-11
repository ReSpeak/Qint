<script lang="typescript">
	// TODO Use scroll-anchoring https://blog.eqrion.net/pin-to-bottom/
	import { onMount } from 'svelte';
	import { get } from "svelte/store";
	import UiMessage from "./UiMessage.svelte";
	import { Chat, Message } from "./chat";
	import Icon from "../ui/Icon.svelte";
	import ClientIcon from "../ui/ClientIcon.svelte";
	import LoadableVirtualList from "../ui/LoadableVirtualList.svelte";
	import LazyList from "../ui/LazyList.svelte";
	import { ListFetchDir, ILazyList } from "../ui/lazyList";
	import { Connection } from "../connection";
	import { assert } from "../util";

	export let connection!: Connection;
	let chat = connection.chat;

	let chatList: ILazyList;
	let messagesError: unknown | undefined;
	let messageInput: HTMLTextAreaElement;

	chat.selectedChat.subscribe(_ => {
		console.log("switch chat");
		if (chatList) {
			chatList.sourceChanged(ListFetchDir.New, ListFetchDir.After);
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

	async function fetchElements(idFrom: Message | undefined, dir: ListFetchDir) {
		messagesError = undefined;
		try {
			console.log("fetching data");
			return chat.getMessages(idFrom, dir);
		} catch (err) {
			console.error("Failed to load messages", err);
			messagesError = err;
			return Chat.EmptyFetch;
		}
	}

	function getItemClient(item: Message) {
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
		console.log("Chatto", chatList);
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
				<div title="{item.date.format('L')}" class="chat-date">
					<div class="chat-date-line"></div>
					<span>{item.date.format('LL')}</span>
					<div class="chat-date-line"></div>
				</div>
			{/if}
			{#if item.displayGroupHeader}
				<div class="invoker-row">
					<div class="invoker-icon chat-left-col">
						<ClientIcon client={item.invoker} {connection} />
					</div>
					<div class="invoker-name has-text-weight-bold">
						<span style={item.clientColor}>{item.displayName}</span>
					</div>
				</div>
			{/if}
			<UiMessage message={item} />
		</LazyList>
	{/if}
	<form class="chat-form" on:submit|preventDefault="{sendMessage}">
		<textarea bind:this={messageInput} bind:value="{chat.composing}" class="input auto_height" name="message"></textarea>
		<button class="button" name="send" type="submit">Send</button>
	</form>
</div>

<style lang="scss">
	.chat {
		overflow: hidden;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		line-height: 1.2;

		// The LazyList
		> :global(.lazyList) {
			flex: 1;
		}
	}

	.chat-form {
		display: flex;
		width: 100%;

		box-shadow: 0px -5px 20px -5px rgba(0, 0, 0, 0.3);
	}

	.chat-form > * {
		box-sizing: border-box;
		height: 2em;
		font-size: 1em;
	}

	.chat-form textarea {
		flex-grow: 1;
		border: {
			top: 1px solid;
			right: 1px solid;
			left: none;
			bottom: none;
			radius: 0;
		}
	}

	.chat-form button {
		background: none;
		color: $blue;
		vertical-align: middle;
		border: {
			top: 1px solid;
			left: none;
			right: none;
			bottom: none;
			radius: 0;
		}
	}

	.chat :global(.scrollPane) {
		padding: 0.5em;
		display: flex;
		flex-direction: column;
	}

	.chat :global(.scrollPane > .lazyListElement) {
		display: flex;
		flex-direction: column;
	}


	@mixin block-margin {
		margin-top: 0.5em;
	}

	.chat-date {
		flex: 1;
		display: flex;
		align-items: center;

		@include block-margin;
		text-align: center;
		color: mix($text, $background, 60%);
	}

	.chat-date-line {
		flex: 1;
		border-top: 1px solid mix($text, $background, 60%);
		margin: { left: 0.5em; right: 0.5em; };
	}

	.invoker-row {
		display: flex;
		align-items: center;
		@include block-margin;
	}

	.chat :global(.chat-left-col) {
		width: 48px;
		padding-right: 8px;
		padding-left: 8px;
	}

	.invoker-icon {
		display: flex;
		justify-content: center;
		text-align: center;
	}

	.invoker-name {
	}
</style>
