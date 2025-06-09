<script lang="ts">
	// Use scroll-anchoring https://blog.eqrion.net/pin-to-bottom/
	import UiMessage from "./Message.svelte";
	import Icon from "../ui/icon/Icon.svelte";
	import TsIcon from "../ui/icon/TsIcon.svelte";
	import ClientName from "../ui/name/ClientName.svelte";
	import ServerName from "../ui/name/ServerName.svelte";
	import LazyList from "../ui/container/LazyList.svelte";
	import ChatInput from "../ui/specialized/ChatInput.svelte";
	import { onDestroy, onMount, tick } from "svelte";
	import { Chat, Message, structuredViewToMd } from "./uiChat";
	import type { MdFile, MdWithFiles } from "./uiChat";
	import { ListFetchDir } from "../ui/container/uiLazyList";
	import { Connection } from "../connection";
	import { app, NodeSelection } from "../app";
	import { Channel, Client, Server } from "../book";
	import { get, writable } from "svelte/store";
	import type { Readable, Writable } from "svelte/store";
	import { assert, on, SERVER_ICON } from "../util";
	import type { ChatData } from "../bookBase";
	import type { ChannelId } from "../ts";
	import { pathJoin } from "../panel/fileUtil";
	import { TsError } from "../book_events";
	import debug from "debug";
	import type { ResultDetails } from "../backend/ws";
	const log = debug("CHAT"),
		error = debug("error:CHAT");

	export let chat: Chat;

	const selections = app.selectedNode;
	$: selected = $selections.getSingleSelection();
	const chatStore = app.settings.chat;

	const developMode = app.settings.ui._developMode;
	let chatList: LazyList<Message> | undefined;
	let messagesError: unknown | undefined;
	let messageInput: ChatInput;
	let text = "";
	let command = "";
	let lastDisplayed: Message | undefined;
	let sendError: string | undefined;
	let isSending = false;
	// The id of the message that is set as last read. (To prevent unnecessary many messages)
	let isSettingLastRead: string | undefined = undefined;

	let canChatHere = false;

	let oldSelection: NodeSelection | undefined;
	let connection: Connection | undefined;
	let ownClient: Writable<Client | undefined>;
	$: {
		connection = selected?.connection;
		ownClient = connection?.book.ownClient ?? writable(undefined);
	}

	let oldOwnChannel: string | undefined;
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
	let chatData: Readable<ChatData> | undefined;
	// Note here: we need to check `chatChanged` and `unreadCountChanged` in one
	// update cycle. Otherwise starting the 'wrong' one first will prevent the
	// second one from working correctly.
	// E.g. unreadCountChanged -> async update stared -> chatChanged -> (does nothing)
	$: {
		chatData = selected?.node.chat;
		if (sel !== selected) {
			sel = selected;
			chatChanged();
		} else if (chatData !== undefined && $chatData) {
			unreadCountChanged();
		}
	}

	$: on(chatList, chatListChanged());

	function chatListChanged() {
		// chatList got mounted
		chatList?.sourceChanged(ListFetchDir.New, ListFetchDir.After);
	}

	function chatChanged() {
		const sel = selected;
		if (!NodeSelection.equals(sel, oldSelection)) {
			if (oldSelection !== undefined) chatStore.save(text, oldSelection);

			oldSelection = sel;
			if (sel === undefined) {
				canChatHere = false;
				chatList?.clear();
			} else {
				text = chatStore.load(sel) ?? "";

				chatList?.sourceChanged(ListFetchDir.New, ListFetchDir.After);
				chatBoxRecheck();
			}
		}
	}

	function unreadCountChanged() {
		chatList?.sourceChanged(ListFetchDir.After, ListFetchDir.After);
	}

	function chatBoxRecheck() {
		const sel = selected;
		if (sel === undefined) {
			canChatHere = false;
			return;
		}
		const ownChannel = $ownClient?.channel;

		const type = sel.node.qlType;
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

	async function sendMessage() {
		try {
			if (isSending) return;
			isSending = true;
			const sel = selected;
			if (sel === undefined) return;
			const textData = messageInput.getStructuredView();
			if (textData.length === 0) return;
			const channelId: ChannelId = get(sel.connection.book.ownClient)?.channel ?? "";
			const mdData = structuredViewToMd(textData, channelId);
			if (!mdData.text) return;
			sendError = await tryUploadChatImage(sel.connection, mdData, channelId);
			if (sendError === undefined) {
				chat.sendMessage(mdData.text);
				messageInput.clear();
				messageInput.focus();
			}
		} finally {
			isSending = false;
		}
	}

	function chatboxContentChanged() {
		sendError = undefined;
	}

	async function chatboxHistoryMove(e: CustomEvent<number>) {
		const historyId = e.detail;
		assert(historyId > 0, "History id should be at least 1");
		if (connection === undefined) return;
		const ownUid = get(connection.book.ownClient)?.uid;
		if (ownUid === undefined || ownUid === null) return;
		const data = await chat.getSendHistory(ownUid, historyId - 1);
		if (data !== undefined) {
			text = data;
			await tick();
			messageInput.moveCursorToEnd();
		}
	}

	async function tryUploadChatImage(
		connection: Connection,
		mdData: MdWithFiles,
		channelId: ChannelId
	): Promise<string | undefined> {
		if (mdData.files.length === 0) return undefined;

		async function tryUpload(file: MdFile) {
			try {
				await connection.backend.upload_bytes(
					{
						cache: false,
						channel: channelId,
						path: pathJoin(file.path, file.name),
					},
					file.blob
				);
				return undefined;
			} catch (err: any) {
				return err as ResultDetails;
			}
		}

		async function createFolder(file: MdFile) {
			return await connection.sendChange({
				ChannelCreateDirectory: {
					id: channelId,
					password: "", // TODO
					path: file.path,
				},
			});
		}

		if (mdData.files.length >= 1) {
			const file = mdData.files[0];
			let uplRes = await tryUpload(file);

			if (uplRes !== undefined && uplRes.tsResult !== TsError.Ok) {
				if (uplRes.tsResult === TsError.PermissionsClientInsufficient) {
					log("No permission to upload file");
					return "No permission to upload files to this channel";
				} else if (
					uplRes.tsResult === TsError.FileInvalidPath ||
					uplRes.tsResult === TsError.FileNotFound
				) {
					log("Creating folder for chat images");
					const cresult = await createFolder(file);
					if (cresult !== undefined) {
						log("Could not create folder %o", cresult);
						return "No permission to create folders in this channel";
					}
					uplRes = await tryUpload(file); // XXX log?
				}
				if (uplRes !== undefined && uplRes.tsResult !== TsError.Ok) {
					log("Unknown error %o", uplRes);
					return `Failed to upload images: ${uplRes.tsResult}`;
				}
			}

			if (mdData.files.length > 1) {
				const tasks = mdData.files.slice(1).map((file) => tryUpload(file));
				await Promise.allSettled(tasks);
			}
		}
		return undefined;
	}

	function sendCommand() {
		if (!command || !connection) return;
		connection.sendMessage({
			SendCommand: {
				command,
			},
		});
		command = "";
	}

	async function fetchElements(idFrom: Message | undefined, dir: ListFetchDir) {
		messagesError = undefined;
		try {
			const res = await chat.getMessages(idFrom, dir);
			return res;
		} catch (err) {
			error("Failed to load messages %o", err);
			messagesError = err;
			return Chat.EmptyFetch;
		}
	}

	async function markRead() {
		if (chatData === undefined || lastDisplayed === undefined) return;
		if (
			document.hasFocus() &&
			lastDisplayed.date.unix() >= $chatData.lastRead.unix() &&
			$chatData.unreadCount > 0 &&
			isSettingLastRead !== lastDisplayed.id
		) {
			isSettingLastRead = lastDisplayed.id;
			await chat.setLastRead(lastDisplayed.id, lastDisplayed.date);
		}
	}

	async function viewchanged(ev: CustomEvent<{ first?: Message; last?: Message }>) {
		lastDisplayed = ev.detail.last;
		await markRead();
	}

	onMount(() => {
		chatChanged();
		if (selected !== undefined) chatList?.sourceChanged(ListFetchDir.New, ListFetchDir.After);
		window.addEventListener("focus", markRead);
	});

	onDestroy(() => {
		// Save current message in chatStore
		if (oldSelection !== undefined) chatStore.save(text, oldSelection);
		window.removeEventListener("focus", markRead);
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
	{:else if selected !== undefined && sel !== undefined}
		<LazyList
			on:viewchanged={viewchanged}
			bind:this={chatList}
			{fetchElements}
			suggestJumpEnd={true}
			notifyViewChanged={chatData !== undefined && $chatData.unreadCount > 0}
			let:item
		>
			<div slot="loading" class="chatFiller">
				<span>Loading ...</span>
				<Icon name="orbit mdi-spin" />
			</div>
			<div slot="empty" class="chatFiller">Chat history empty ¯\_(ツ)_/¯</div>
			{#if item.displayDateSeparator}
				<div title={item.date.format("L")} class="chat-date">
					<div class="chat-date-line" />
					<span>{item.date.format("LL")}</span>
					<div class="chat-date-line" />
				</div>
			{/if}
			{#if item.displayGroupHeader}
				<div class="invoker-row">
					<div class="invoker-icon chat-left-col">
						{#if item.invoker}
							<TsIcon type="client" source={item.invoker} {connection} />
						{:else}
							<Icon name={SERVER_ICON} />
						{/if}
					</div>
					<div class="invoker-name has-text-weight-bold">
						{#if item.invoker}
							<ClientName client={item.invoker} />
						{:else}Server{/if}
					</div>
				</div>
			{/if}
			<UiMessage
				message={item}
				unread={chatData !== undefined && item.date > $chatData.lastRead}
				connection={sel.connection}
			/>
		</LazyList>
		<form class="chat-form" class:hidden={!canChatHere} on:submit|preventDefault={sendMessage}>
			{#if sendError !== undefined}
				<div class="sendError">{sendError}</div>
			{/if}
			<div class="sendCombo">
				<ChatInput
					bind:this={messageInput}
					bind:value={text}
					hasHistory={true}
					on:submit={sendMessage}
					on:structureChanged={chatboxContentChanged}
					on:historyMove={chatboxHistoryMove}
				>
					<svelte:fragment slot="placeholder">
						<span>Send&nbsp;to&nbsp;</span>
						{#if sel.node instanceof Client}
							<ClientName client={sel.node} />
						{:else if sel.node instanceof Channel}
							<span>your channel</span>
						{:else if sel.node instanceof Server}
							<ServerName server={sel.node} {connection} />
						{/if}
					</svelte:fragment>
				</ChatInput>
				<button
					class="button outline-button"
					class:is-loading={isSending}
					name="send"
					type="submit"
					style="height: auto;">Send</button
				>
			</div>
		</form>
		{#if $developMode}
			<form class="chat-form" on:submit|preventDefault={sendCommand}>
				<div class="sendCombo">
					<ChatInput bind:value={command} />
					<button class="button" name="send" type="submit" style="height: auto;"
						>Send Command</button
					>
				</div>
			</form>
		{/if}
	{:else}
		<div class="chatFiller">No chat selected</div>
	{/if}
</div>

<style lang="scss">
	@use "../index.scss" as *;
	@import "./chat_style";

	.invoker-row {
		@include invoker-row;
	}

	.chat :global(.chat-left-col) {
		@include chat-left-col;
		width: 48px;
	}

	.chat {
		overflow: hidden;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		line-height: initial;
		border-right: none;

		// For drop overlay
		position: relative;

		// The LazyList
		> :global(.lazyList) {
			flex: 1;
		}
	}

	.chat-form {
		display: flex;
		flex-direction: column;
		height: auto;

		margin: 0.5em;
		margin-top: 0;

		:global(img) {
			max-height: min(50vh, 100%);
			max-width: min(50vw, 100%);
		}
	}

	.sendCombo {
		display: flex;
		box-shadow: 0px -5px 20px -5px rgba(0, 0, 0, 0.3);
		max-height: 50vh;
	}

	.sendError {
		border-radius: 0.25em;
		background-color: $red;
		margin-bottom: 0.5em;
		padding: 0.25em;
	}

	.chat :global(.scrollPane:last-child) {
		padding-bottom: 0.5em;
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
