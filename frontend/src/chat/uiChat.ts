import { get, Readable } from "svelte/store";
import type { Moment } from "moment";
import { GraphQlClient, ChatData } from "../book";
import { datetimeDeserialize, getDataColor, assert, Lazy } from "../util";
import { ListFetchDir, FetchResult } from "../ui/container/uiLazyList";
import { NodeSelection } from "../app";
import { backend } from "../backend/backend";
import { StructuredData } from "../ui/specialized/uiChatInput";
import { ChannelId, Uid } from "../ts";
import moment from "moment";
import debug from "debug";
const log = debug("CHAT"),
	error = debug("error:CHAT");

export class Chat {
	public static readonly EmptyFetch: FetchResult<Message> = {
		items: [],
		canLoadBeforeStart: false,
		canLoadAfterEnd: false,
	};

	public constructor(private readonly selectedChat: Readable<NodeSelection | undefined>) {}

	private static groupMessages(
		messages: Message[],
		lastEntry: Message | undefined,
		dir: ListFetchDir
	): void {
		let previousMessage: Message | undefined;

		if (lastEntry) {
			if (dir === ListFetchDir.Before) {
				lastEntry.displayGroupHeader = false;
				lastEntry.displayDateSeparator = false;
				messages.push(lastEntry);
			} else if (dir === ListFetchDir.After) {
				previousMessage = lastEntry;
			}
		}

		for (const message of messages) {
			const previousDate = previousMessage?.date;
			message.displayDateSeparator =
				!previousDate || !previousDate.isSame(message.date, "day");
			message.displayGroupHeader =
				message.displayDateSeparator ||
				!previousMessage ||
				!GraphQlClient.equals(previousMessage.invoker, message.invoker);
			previousMessage = message;
		}

		if (lastEntry) {
			if (dir === ListFetchDir.Before) messages.pop();
		}
	}

	public async getMessages(
		idFrom: Message | undefined,
		dir: ListFetchDir
	): Promise<FetchResult<Message>> {
		const selected = get(this.selectedChat);
		if (selected === undefined) return Chat.EmptyFetch;
		const publicKey = selected.connection.book.server.publicKey;
		if (publicKey === undefined) {
			error("Cannot get messages for a non-existant connection");
			return Chat.EmptyFetch;
		}

		let startTime;
		let startId;
		let loadAtBeginning: boolean | undefined;
		switch (dir) {
			case ListFetchDir.Before:
				loadAtBeginning = true;
				break;
			case ListFetchDir.New:
				loadAtBeginning = undefined;
				break;
			case ListFetchDir.After:
				loadAtBeginning = false;
				break;
			default:
				assert(false, "Unknown direction");
		}

		if (idFrom) {
			startTime = idFrom.date.unix();
			startId = idFrom.id;
		}

		const res = await backend.graphql(
			`query GetMessages($chatType: GMessageTarget!, $server: [Int!]!, $chatId: ID,
					$startTime: NaiveDateTime, $startId: ID, $loadAtBeginning: Boolean) {
				chat(typ: $chatType, server: $server, id: $chatId) {
					messages(startTime: $startTime, startId: $startId, beforeStart: $loadAtBeginning) {
						id
						invoker {
							client {
								uid
								name
								customName
							}
							icon
							avatar
						}
						invokerName
						content
						rendered
						status
						isPoke
						time
						timezone
					}
				}
			}`,
			{
				chatType: selected.node.qlType,
				server: publicKey,
				chatId: selected.node.qlId,
				startTime,
				startId,
				loadAtBeginning,
			}
		);
		if ("data" in res) {
			// We never chatted here
			if (!res.data.chat || res.data.chat.messages.length === 0) {
				log("No chats here");
				return Chat.EmptyFetch;
			}

			const msgs: Message[] = [];
			res.data.chat.messages.forEach((msg: any) => {
				let client;
				if (msg.invoker) {
					client = GraphQlClient.fromGraphqlInvoker(msg.invoker);
				}
				msgs.push(
					new Message(
						msg.id,
						client,
						msg.invokerName,
						msg.content,
						msg.rendered,
						datetimeDeserialize([msg.time, msg.timezone]),
						msg.status,
						msg.isPoke
					)
				);
			});
			log(
				"Fetching messages " + (loadAtBeginning ? "before" : "after"),
				[startTime, startId],
				"; got",
				msgs
			);

			Chat.groupMessages(msgs, idFrom, dir);

			return {
				items: msgs,
				canLoadBeforeStart: true,
				canLoadAfterEnd: dir !== ListFetchDir.New, // Heuristic: when fetching new we start at the end
			};
		} else {
			error("GetMessages result does not contain data", res);
			return Chat.EmptyFetch;
		}
	}

	public sendMessage(message: string): void {
		const selected = get(this.selectedChat);
		if (selected === undefined) return;
		selected.connection.sendMessage({
			SendMessage: {
				target: selected.node.wsTarget,
				message,
			},
		});
	}

	public async setLastRead(messageId: string, lastRead: Moment): Promise<void> {
		const selected = get(this.selectedChat);
		if (selected === undefined) return;
		const publicKey = selected.connection.book.server.publicKey;
		if (publicKey === undefined) {
			error("Cannot get messages for a non-existant connection");
			return;
		}
		const res = await backend.graphql(
			`mutation SetLastRead($chatType: GMessageTarget!, $server: [Int!]!, $chatId: ID,
					$message: ID!) {
				setLastRead(typ: $chatType, server: $server, id: $chatId, message: $message)
			}`,
			{
				chatType: selected.node.qlType,
				server: publicKey,
				chatId: selected.node.qlId,
				message: messageId,
			}
		);
		selected.node.updateChat(new ChatData(lastRead, res.data.setLastRead));
	}

	public async getSendHistory(from: Uid, id: number): Promise<string | undefined> {
		const selected = get(this.selectedChat);
		if (selected === undefined) return undefined;
		const publicKey = selected.connection.book.server.publicKey;
		if (publicKey === undefined) {
			error("Cannot get send history for a non-existant connection");
			return undefined;
		}

		const res = await backend.graphql<{ chat: { sendHistory: { content: string } } }>(
			`query GetSendHistory($chatType: GMessageTarget!, $server: [Int!]!, $chatId: ID,
					$from: [Int!]!, $id: Int!) {
				chat(typ: $chatType, server: $server, id: $chatId) {
					sendHistory(from: $from, id: $id) {
						content
					}
				}
			}`,
			{
				chatType: selected.node.qlType,
				server: publicKey,
				chatId: selected.node.qlId,
				from,
				id,
			}
		);
		if ("data" in res) {
			// We never chatted here
			if (!res.data.chat || !res.data.chat.sendHistory) {
				log("No messages here");
				return undefined;
			}

			return res.data.chat.sendHistory.content;
		} else {
			error("GetSendHistory result does not contain data", res);
			return undefined;
		}
	}
}

export const enum MessageStatus {
	Sending = "SENDING",
	Success = "SUCCESS",
	Error = "ERROR",
}

export class Message {
	private readonly _clientColor: Lazy<string>;
	public displayDateSeparator: boolean = false;
	public displayGroupHeader: boolean = false;

	public get displayName(): string {
		return this.invoker?.name ?? this.invokerName ?? "";
	}
	public get clientColor(): string {
		return this._clientColor.get();
	}

	constructor(
		public id: string,
		public invoker: GraphQlClient | undefined,
		public invokerName: string | undefined,
		public raw: string,
		public rendered: string,
		public date: Moment,
		public status: MessageStatus,
		public isPoke: boolean
	) {
		this._clientColor = new Lazy(() => this.generateClientColor());
	}

	private generateClientColor(): string {
		if (this.invoker?.uid) {
			return getDataColor(this.invoker.uid);
		} else {
			return getDataColor(this.displayName);
		}
	}

	public hasSameInvoker(other: Message): boolean {
		return Message.hasSameInvoker(this, other);
	}

	// TODO recheck
	public static hasSameInvoker(first: Message, second: Message): boolean {
		if (first.invoker === second.invoker) return true;
		if (first.invoker === undefined || second.invoker === undefined) {
			return first.invokerName === second.invokerName;
		}
		return first.invoker.equals(second.invoker);
	}
}

export interface MdWithFiles {
	text: string;
	files: MdFile[];
}
export interface MdFile {
	path: string;
	name: string;
	blob: Blob;
}

const QINT_CHAT_FOLDER = "/.qint_chat";

export function structuredViewToMd(data: StructuredData, channel: ChannelId): MdWithFiles {
	let text = "";
	let chainid = 0;
	let date: { iso: string; unix: string } | undefined;
	const files = [];
	for (const part of data) {
		if (typeof part === "string") {
			text += part;
		} else if ("src" in part) {
			text += `![](${part.src})`;
		} else if ("blob" in part) {
			const blob = part.blob;
			if (date === undefined) {
				const m = moment();
				date = { iso: m.format("YYYY-MM-DD_HH-mm-ss-SSS"), unix: m.unix().toString() };
			}
			const file = part.blob.type === "image/jpeg" ? "jpg" : "png";
			const name = `${date.iso}-${chainid++}.${file}`;
			files.push({ blob, path: QINT_CHAT_FOLDER, name });
			const tslink = `ts3file://server?port=${0}&serverUID=${""}&channel=${channel}&path=${encodeURIComponent(
				QINT_CHAT_FOLDER
			)}&filename=${encodeURIComponent(name)}&isDir=0&size=${blob.size}&fileDateTime=${
				date.unix
			}`;
			text += `![](${tslink})`;
		}
	}
	return {
		text,
		files,
	};
}
