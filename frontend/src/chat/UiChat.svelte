<script lang="typescript">
	// TODO Use scroll-anchoring https://blog.eqrion.net/pin-to-bottom/
	import { onMount, tick } from "svelte";
	import UiMessage from "./UiMessage.svelte";
	import { Chat, Message } from "./chat";
	import Icon from "../ui/Icon.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import LazyList from "../ui/LazyList.svelte";
	import { ListFetchDir } from "../ui/lazyList";
	import { Connection } from "../connection";
	import BInput from "../ui/BInput.svelte";
	import { on } from "../util";
	import { app } from "../app";
	import { MessageTarget } from "../ts";
	import { Client } from "../book";
	import { writable } from "svelte/store";
	import type { Writable } from "svelte/store";

	export let chat: Chat;

	const selected = app.selectedNode;
	let chatStore = app.transientSettings.chat;

	let chatList: LazyList;
	let messagesError: unknown | undefined;
	let messageInput: BInput;
	let text = "";

	let canChatHere = true;

	let oldSelectedChat: MessageTarget | undefined = undefined;
	let unreadCount = chat.unreadCount;
	let connection: Connection | undefined;
	let ownClient: Writable<Client | undefined>;
	$: {
		connection = $selected?.connection;
		ownClient = connection?.book.ownClient ?? writable(undefined);
	}

	$: on($selected, chatChanged());
	$: on($unreadCount, chatEndChanged());
	let oldOwnChannel: number | undefined;
	let oldCon: string | undefined;
	$: {
		const ownChannel = $ownClient?.channel;
		const con = connection?.backend.id;
		if (ownChannel !== oldOwnChannel || con !== oldCon) {
			oldOwnChannel = ownChannel;
			oldCon = con;
			chatBoxRecheck();
		}
	}

	function chatChanged() {
		const sel = $selected;
		if (sel === undefined) {
			oldSelectedChat = undefined;
			chatList.clear();
			canChatHere = false;
			return;
		}
		let { connection, target } = sel;
		if (oldSelectedChat !== undefined) chatStore.set(text, oldSelectedChat, connection);
		text = chatStore.get(target, connection) ?? "";

		if (target !== oldSelectedChat) {
			oldSelectedChat = target;

			if (chatList) {
				chatList.sourceChanged(ListFetchDir.New, ListFetchDir.After);
			}

			chatBoxRecheck();
		}
	}

	function chatEndChanged() {
		if (chatList) {
			chatList.sourceChanged(ListFetchDir.After, ListFetchDir.After);
		}
	}

	function chatBoxRecheck() {
		const ownChannel = $ownClient?.channel;
		const sel = $selected;
		if (sel === undefined) {
			canChatHere = false;
			return;
		}
		let { target } = sel;

		if ("Server" in target || "Client" in target) {
			canChatHere = true;
		} else if ("Channel" in target) {
			canChatHere = ownChannel === undefined || target.Channel === ownChannel;
		}
		if (canChatHere) {
			(async () => {
				await tick();
				if (messageInput) {
					messageInput.focus();
				}
			})();
		}
	}

	function sendMessage() {
		if (!text) return;
		chat.sendMessage(text);
		text = "";
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
			return chat.getMessages(idFrom, dir);
		} catch (err) {
			console.error("Failed to load messages", err);
			messagesError = err;
			return Chat.EmptyFetch;
		}
	}

	onMount(() => {
		chatChanged();
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
			<div slot="loading" class="chatFiller">
				<span>Loading ...</span>
				<Icon name="orbit mdi-spin" />
			</div>
			<div slot="empty" class="chatFiller">Chat history empty ¯\_(ツ)_/¯</div>
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
	<form class="chat-form" class:hidden={!canChatHere} on:submit|preventDefault={sendMessage}>
		<BInput bind:this={messageInput} bind:value={text} on:keydown={onChatKeyDown} />
		<button class="button" name="send" type="submit" style="height: auto;">Send</button>
	</form>
</div>

<style lang="scss">
	.chat {
		overflow: hidden;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		line-height: 1.2;
		border-right: none;

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

	.chatFiller {
		@extend %unselectable;
		width: 100%;
		height: 100%;
		padding: 0 1em 3em 1em;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		align-items: center;
		color: gray;
		font-size: xx-large;
		white-space: nowrap;

		:global(.icon) {
			font-size: 72px;
		}
	}
</style>
