<script lang="ts">
	import UiMessage from "./Message.svelte";
	import ChatModeSwitch from "./ChatModeSwitch.svelte";
	import Icon from "../ui/icon/Icon.svelte";
	import LazyList from "../ui/container/LazyList.svelte";
	import ChatInput from "../ui/specialized/ChatInput.svelte";
	import { onDestroy } from "svelte";
	import moment from "moment";
	import { Message, MessageStatus } from "./uiChat";
	import { ListFetchDir } from "../ui/container/uiLazyList";
	import type { FetchResult } from "../ui/container/uiLazyList";
	import { backend } from "../backend/backend";
	import type { MatrixMessage } from "../backend/backend";
	import { matrixMessages, matrixStatus } from "../matrix";
	import debug from "debug";
	const error = debug("error:MATRIX");

	const PAGE = 30;

	let chatList: LazyList<Message> | undefined;
	let messageInput: ChatInput;
	let text = "";
	let messagesError: unknown | undefined;
	let sendError: string | undefined;
	let isSending = false;

	// Everything loaded so far for the current room, oldest first.
	// LazyList may drop and refetch items, so it is served from here.
	let cache: Message[] = [];
	let endToken: string | undefined;
	let historyDone = false;
	let loadedRoom: string | undefined;

	$: roomId = $matrixStatus.roomId;
	$: roomName = $matrixStatus.rooms.find((r) => r.id === roomId)?.name ?? roomId;
	$: if (roomId !== loadedRoom) roomChanged();
	// Initial load once the list got mounted
	$: chatList?.sourceChanged(ListFetchDir.New, ListFetchDir.After);

	function roomChanged() {
		loadedRoom = roomId;
		cache = [];
		endToken = undefined;
		historyDone = false;
		chatList?.sourceChanged(ListFetchDir.New, ListFetchDir.After);
	}

	function toMessage(m: MatrixMessage): Message {
		return new Message(
			m.id,
			undefined,
			m.senderName,
			m.body,
			m.rendered,
			moment(m.time),
			MessageStatus.Success,
			false
		);
	}

	function regroup() {
		let prev: Message | undefined;
		for (const m of cache) {
			m.displayDateSeparator = !prev || !prev.date.isSame(m.date, "day");
			m.displayGroupHeader = m.displayDateSeparator || !prev || !prev.hasSameInvoker(m);
			prev = m;
		}
	}

	async function loadOlder() {
		const room = loadedRoom;
		const res = await backend.matrix_messages(endToken);
		if (room !== loadedRoom) return;
		const known = new Set(cache.map((m) => m.id));
		const older = res.messages
			.map(toMessage)
			.reverse()
			.filter((m) => !known.has(m.id));
		cache = [...older, ...cache];
		endToken = res.endToken;
		if (endToken === undefined || endToken === null) historyDone = true;
		regroup();
	}

	const indexOf = (m: Message | undefined) => cache.findIndex((c) => c.id === m?.id);

	async function getMessages(idFrom: Message | undefined, dir: ListFetchDir): Promise<FetchResult<Message>> {
		if (roomId === undefined) {
			return { items: [], canLoadBeforeStart: false, canLoadAfterEnd: false };
		}
		if (dir === ListFetchDir.After) {
			const start = indexOf(idFrom) + 1;
			const end = start + PAGE;
			return { items: cache.slice(start, end), canLoadBeforeStart: false, canLoadAfterEnd: end < cache.length };
		}
		// Before and New: page through history until we have enough text messages
		let idx = dir === ListFetchDir.New ? cache.length : indexOf(idFrom);
		if (idx === -1) return { items: [], canLoadBeforeStart: false, canLoadAfterEnd: false };
		while (idx < PAGE && !historyDone) {
			const oldLen = cache.length;
			await loadOlder();
			idx += cache.length - oldLen;
		}
		const start = Math.max(0, idx - PAGE);
		return {
			items: cache.slice(start, idx),
			canLoadBeforeStart: start > 0 || !historyDone,
			canLoadAfterEnd: false,
		};
	}

	async function fetchElements(idFrom: Message | undefined, dir: ListFetchDir) {
		messagesError = undefined;
		try {
			return await getMessages(idFrom, dir);
		} catch (err) {
			error("Failed to load messages %o", err);
			messagesError = err;
			return { items: [], canLoadBeforeStart: false, canLoadAfterEnd: false };
		}
	}

	const unsubscribe = matrixMessages.subscribe((m) => {
		if (m.roomId !== loadedRoom || cache.some((c) => c.id === m.id)) return;
		cache = [...cache, toMessage(m)];
		regroup();
		chatList?.sourceChanged(ListFetchDir.After, ListFetchDir.After);
	});
	onDestroy(unsubscribe);

	async function sendMessage() {
		if (isSending) return;
		// Text only for now, pasted images are dropped
		const msg = messageInput
			.getStructuredView()
			.filter((p): p is string => typeof p === "string")
			.join("");
		if (!msg.trim()) return;
		isSending = true;
		try {
			await backend.matrix_send(msg);
			messageInput.clear();
			messageInput.focus();
		} catch (err) {
			sendError = String(err);
		} finally {
			isSending = false;
		}
	}
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
	{:else if roomId !== undefined}
		<LazyList bind:this={chatList} {fetchElements} suggestJumpEnd={true} let:item>
			<div slot="loading" class="chatFiller">
				<span>Loading ...</span>
				<Icon name="orbit mdi-spin" />
			</div>
			<div slot="empty" class="chatFiller">Chat history empty ¯\\_(ツ)_/¯</div>
			{#if item.displayDateSeparator}
				<div title={item.date.format("L")} class="chat-date">
					<div class="chat-date-line"></div>
					<span>{item.date.format("LL")}</span>
					<div class="chat-date-line"></div>
				</div>
			{/if}
			{#if item.displayGroupHeader}
				<div class="invoker-row">
					<div class="invoker-icon chat-left-col">
						<Icon name="matrix" />
					</div>
					<div class="invoker-name has-text-weight-bold">{item.displayName}</div>
				</div>
			{/if}
			<UiMessage message={item} connection={undefined} />
		</LazyList>
	{:else}
		<div class="chatFiller">No Matrix room selected</div>
	{/if}
	<form class="chat-form" on:submit|preventDefault={sendMessage}>
		{#if sendError !== undefined}
			<div class="sendError">{sendError}</div>
		{/if}
		<div class="sendCombo">
			<ChatInput
				bind:this={messageInput}
				bind:value={text}
				on:submit={sendMessage}
				on:structureChanged={() => (sendError = undefined)}
			>
				<svelte:fragment slot="placeholder">
					<span>Send&nbsp;to&nbsp;{roomName ?? "Matrix"}</span>
				</svelte:fragment>
			</ChatInput>
			<div class="sendButtons">
				<button
					class="button outline-button"
					class:is-loading={isSending}
					name="send"
					type="submit"
					title="Send"
					aria-label="Send"
					disabled={roomId === undefined}
				>
					<Icon name="send" />
				</button>
				<ChatModeSwitch />
			</div>
		</div>
	</form>
</div>

<style lang="scss">
	@import "./chat_panel";
</style>
