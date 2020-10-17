<script lang="typescript">
	// TODO Use scroll-anchoring https://blog.eqrion.net/pin-to-bottom/
	import UiMessage from "./UiMessage.svelte";
	import Icon from "../ui/Icon.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import UiLazyList from "../ui/UiLazyList.svelte";
	import BInput from "../ui/BInput.svelte";
	import { onMount, tick } from "svelte";
	import { Chat, Message } from "./chat";
	import { ListFetchDir } from "../ui/lazyList";
	import { Connection } from "../connection";
	import { app, NodeSelection } from "../app";
	import { Channel, Client, Server } from "../book";
	import { writable } from "svelte/store";
	import type { Writable } from "svelte/store";
	import { on } from "../util";

	export let chat: Chat;

	const selected = app.selectedNode;
	let chatStore = app.transientSettings.chat;

	let chatList: UiLazyList | undefined;
	let messagesError: unknown | undefined;
	let messageInput: BInput;
	let text = "";

	let canChatHere = false;

	let oldSelection: NodeSelection | undefined = undefined;
	let connection: Connection | undefined;
	let ownClient: Writable<Client | undefined>;
	$: {
		connection = $selected?.connection;
		ownClient = connection?.book.ownClient ?? writable(undefined);
	}

	$: chatData = sel?.node.chat;
	$: on(chatData !== undefined && $chatData, unreadCountChanged());

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

	let sel: NodeSelection | undefined;
	$: {
		sel = $selected;
		chatChanged();
	}

	function chatChanged() {
		if (!chatList) return;

		const sel = $selected;
		if (sel === undefined) {
			oldSelection = undefined;
			chatList?.clear();
			canChatHere = false;
			return;
		}

		if (!NodeSelection.equals(sel, oldSelection)) {
			if (oldSelection !== undefined) chatStore.save(text, oldSelection);
			text = chatStore.load(sel) ?? "";
			oldSelection = sel;

			chatList.sourceChanged(ListFetchDir.New, ListFetchDir.After);
			chatBoxRecheck();
		}
	}

	function unreadCountChanged() {
		if (!chatList) return;

		chatList.sourceChanged(ListFetchDir.After, ListFetchDir.After);
	}

	function chatBoxRecheck() {
		const sel = $selected;
		if (sel === undefined) {
			canChatHere = false;
			return;
		}
		const ownChannel = $ownClient?.channel;

		let type = sel.node.qlType;
		if (type === "SERVER" || type === "CLIENT") {
			canChatHere = true;
		} else if (type === "CHANNEL" && sel.node instanceof Channel) {
			canChatHere = ownChannel === undefined || sel.node.id === ownChannel;
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
			const res = await chat.getMessages(idFrom, dir);
			return res;
		} catch (err) {
			console.error("Failed to load messages", err);
			messagesError = err;
			return Chat.EmptyFetch;
		}
	}

	// TODO on got focus
	async function viewchanged(ev: CustomEvent<{ first?: Message; last?: Message }>) {
		if (chatData === undefined || ev.detail.last === undefined) return;
		let lastDisplayed = ev.detail.last;
		if (lastDisplayed.date > $chatData.lastRead) {
			await chat.setLastRead(lastDisplayed.id, lastDisplayed.date);
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
	{:else if sel !== undefined}
		<UiLazyList
			on:viewchanged={viewchanged}
			bind:this={chatList}
			{fetchElements}
			suggestJumpEnd={true}
			notifyViewChanged={chatData !== undefined && $chatData.unreadCount > 0}
			let:item>
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
			<UiMessage
				message={item}
				unread={chatData !== undefined && item.date > $chatData.lastRead} />
		</UiLazyList>
		<form class="chat-form" class:hidden={!canChatHere} on:submit|preventDefault={sendMessage}>
			<BInput bind:this={messageInput} bind:value={text} on:keydown={onChatKeyDown}>
				<div slot="placeholder">
					<span>Send to</span>
					<!-- TODO: Remove 'sel !== undefined' when svelte-tool understands it -->
					{#if sel !== undefined && sel.node instanceof Client}
						<ClientName client={sel.node} />
					{:else if sel !== undefined && sel.node instanceof Channel}
						<span> your channel</span>
					{:else if sel !== undefined && sel.node instanceof Server}
						<ServerName connection={sel.connection} />
					{/if}
				</div>
			</BInput>
			<button class="button" name="send" type="submit" style="height: auto;">Send</button>
		</form>
	{:else}
		<div class="chatFiller">No chat selected</div>
	{/if}
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

		:global(a:hover) {
			text-decoration: underline;
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
