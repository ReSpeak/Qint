<script lang="typescript">
	// TODO Use scroll-anchoring https://blog.eqrion.net/pin-to-bottom/
	import { onMount } from "svelte";
	import { get } from "svelte/store";
	import UiMessage from "./UiMessage.svelte";
	import { Chat, Message } from "./chat";
	import TsIcon from "../ui/TsIcon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import LazyList from "../ui/LazyList.svelte";
	import { ListFetchDir } from "../ui/lazyList";
	import { Connection } from "../connection";
	import BInput from "../ui/BInput.svelte";

	export let connection: Connection;
	let chat = connection.chat;

	let chatList: LazyList;
	let messagesError: unknown | undefined;
	let messageInput: BInput;

	let canChatHere = true;

	chat.selectedChat.subscribe((c) => {
		console.log("switch chat");
		if (chatList) {
			chatList.sourceChanged(ListFetchDir.New, ListFetchDir.After);
		}
		if (messageInput) messageInput.focus();

		if ("Server" in c || "Client" in c) {
			canChatHere = true;
		} else if ("Channel" in c) {
			canChatHere = c.Channel === get(connection.ownClient)!.channel;
		}
	});

	chat.unreadCount.subscribe((_) => {
		if (chatList) chatList.sourceChanged(ListFetchDir.After, ListFetchDir.After);
	});

	function sendMessage() {
		if (!chat.composing) return;
		chat.sendMessage();
		chat.composing = "";
		messageInput.focus();
	}

	function onChatKeyDown(e: any) {
		if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey) {
			sendMessage();
			e.preventDefault();
		}
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

	onMount(() => {
		chatList.sourceChanged(ListFetchDir.New, ListFetchDir.After);
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
				<div class="message-body">Failed to fetch messages</div>
			</article>
		</div>
	{:else}
		<LazyList bind:this={chatList} {fetchElements} suggestJumpEnd={true} let:item>
			<div slot="loading" class="loader" />
			{#if item.displayDateSeparator}
				<div title={item.date.format('L')} class="chat-date">
					<div class="chat-date-line" />
					<span>{item.date.format('LL')}</span>
					<div class="chat-date-line" />
				</div>
			{/if}
			{#if item.displayGroupHeader}
				<div class="invoker-row">
					<div class="invoker-icon chat-left-col">
						<TsIcon type="client" source={item.invoker} {connection} />
					</div>
					<div class="invoker-name has-text-weight-bold">
						<ClientName client={item.invoker} />
					</div>
				</div>
			{/if}
			<UiMessage message={item} />
		</LazyList>
	{/if}
	{#if canChatHere}
		<form class="chat-form" on:submit|preventDefault={sendMessage}>
			<BInput
				bind:this={messageInput}
				bind:value={chat.composing}
				on:keydown={onChatKeyDown} />
			<button class="button" name="send" type="submit" style="height: auto;">Send</button>
		</form>
	{/if}
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
		height: auto;

		margin: 0.5em;
		margin-top: 0;
	}

	.chat-form > :global(*) {
		box-shadow: 0px -5px 20px -5px rgba(0, 0, 0, 0.3);
	}

	.chat-form button {
		background: none;
		color: $blue;
		vertical-align: middle;
		border: 1px solid;
		border-radius: 5px;
		margin-left: 0.5em;
	}

	.chat :global(.scrollPane) {
		padding: 0.5em;
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
		margin: {
			left: 0.5em;
			right: 0.5em;
		}
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

	// .invoker-name {
	// }
</style>
