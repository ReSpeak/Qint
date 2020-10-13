<script lang="typescript">
	// TODO Use scroll-anchoring https://blog.eqrion.net/pin-to-bottom/
	import UiMessage from "./UiMessage.svelte";
	import Icon from "../ui/Icon.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import LazyList from "../ui/LazyList.svelte";
	import BInput from "../ui/BInput.svelte";
	import { onMount, tick } from "svelte";
	import { Chat, Message } from "./chat";
	import { ListFetchDir } from "../ui/lazyList";
	import { Connection } from "../connection";
	import { app, NodeSelection } from "../app";
	import { Channel, Client, Server } from "../book";
	import { writable } from "svelte/store";
	import type { Writable } from "svelte/store";
	import { assert, binarySearchByKey, on } from "../util";

	export let chat: Chat;

	const selected = app.selectedNode;
	let chatStore = app.transientSettings.chat;

	let chatList: LazyList | undefined;
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

	$: chatData = sel?.node.getChat();
	$: on(chatData !== undefined && $chatData, unreadCountChanged(), checkChatScroll());

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

	function getDisplayed(): Message[] {
		if (!chatList) return [];
		const elems = chatList.getElements();
		const scrollElem = chatList.getScrollElement();
		const htmlElems = chatList.getHtmlElements();
		const scrollTop = scrollElem.scrollTop;
		const height = scrollElem.clientHeight;
		assert(elems.length === htmlElems.length, "HTML node count does not match message count");
		if (elems.length === 0) return [];

		const distFn = (e: HTMLElement) => {
			// The bottom of the element within our list (unscrolled)
			const bottomStaticOffset = e.offsetTop;
			// The top of the element without our list (with scroll offset)
			const bottomCurrentOffset = bottomStaticOffset - scrollTop;
			return bottomCurrentOffset;
		};

		// Where would we need to insert an element that starts one pixel from the top
		let res = binarySearchByKey(htmlElems, 1, distFn);
		const start = res.index > 0 ? res.index - 1 : res.index;
		// Where would we need to insert an element that starts at the bottom
		res = binarySearchByKey(htmlElems, height, distFn);
		const end = res.index;
		return elems.slice(start, end);
	}

	function chatChanged() {
		if (!chatList) return;

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

	function checkChatScroll() {
		if (chatData !== undefined && chatList !== undefined) {
			const scrollElem = chatList.getScrollElement();
			const chat = $chatData;
			if (chat.unreadCount > 0) {
				scrollElem.addEventListener("scroll", onChatScroll);
				onChatScroll();
			} else {
				scrollElem.removeEventListener("scroll", onChatScroll);
			}
		}
	}

	// If unread chat messages are visible, register a mouse move handler
	async function onChatScroll() {
		if (chatData === undefined || chatList === undefined) return;
		// Wait for html elements to update, e.g. when the chat changed
		await tick();
		const chat = $chatData;
		const scrollElem = chatList.getScrollElement();
		let displayedUnreadCount = getDisplayed().reduce(
			(sum, msg) => sum + (msg.date > chat.lastRead ? 1 : 0),
			0
		);
		if (displayedUnreadCount > 0) {
			scrollElem.addEventListener("mousemove", onMouseMove);
		} else {
			scrollElem.removeEventListener("mousemove", onMouseMove);
		}
	}

	// Mark all currently visible chat messages as read
	function onMouseMove() {
		const displayed = getDisplayed();
		if (chatData === undefined || chatList === undefined || displayed.length === 0) return;
		const chatDat = $chatData;
		let lastDisplayed = displayed[displayed.length - 1];
		if (lastDisplayed.date > chatDat.lastRead) {
			chat.setLastRead(lastDisplayed.id, lastDisplayed.date);
			const scrollElem = chatList.getScrollElement();
			scrollElem.removeEventListener("mousemove", onMouseMove);
		}
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
			setTimeout(checkChatScroll);
			return res;
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
	{:else if sel !== undefined}
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
			<UiMessage
				message={item}
				unread={chatData !== undefined && item.date > $chatData.lastRead} />
		</LazyList>
		<form class="chat-form" class:hidden={!canChatHere} on:submit|preventDefault={sendMessage}>
			<BInput bind:this={messageInput} bind:value={text} on:keydown={onChatKeyDown}>
				<div slot="placeholder">
					<span>Send to</span>
					{#if sel.node instanceof Client}
						<ClientName client={sel.node} />
					{:else if sel.node instanceof Channel}
						<span> your channel</span>
					{:else if sel.node instanceof Server}
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
