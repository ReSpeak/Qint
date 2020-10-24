import { get, writable, Writable, Readable } from "svelte/store";
import type { Moment } from "moment";
import { graphql } from "../graphql";
import { GraphQlClient, ChatData } from "../book";
import { datetimeDeserialize, getDataColor, assert, Lazy } from "../util";
import { ListFetchDir, FetchResult } from "../ui/lazyList";
import { NodeSelection } from "../app";

export class Chat {
	public static readonly EmptyFetch: FetchResult<Message> = {
		items: [],
		canLoadBeforeStart: false,
		canLoadAfterEnd: false
	};

	public constructor(
		private readonly selectedChat: Readable<NodeSelection | undefined>) { }

	private static groupMessages(messages: Message[], lastEntry: Message | undefined, dir: ListFetchDir): void {
		let previousMessage: Message | undefined;

		if (lastEntry) {
			if (dir === ListFetchDir.Before) {
				lastEntry.displayGroupHeader = false;
				lastEntry.displayDateSeparator = false;
				messages.push(lastEntry);
			}
			else if (dir === ListFetchDir.After) {
				previousMessage = lastEntry;
			}
		}

		for (const message of messages) {
			const previousDate = previousMessage?.date;
			message.displayGroupHeader = !previousMessage || !GraphQlClient.equals(previousMessage.invoker, message.invoker);
			message.displayDateSeparator = !previousDate || !previousDate.isSame(message.date, "day");
			previousMessage = message;
		}

		if (lastEntry) {
			if (dir === ListFetchDir.Before) messages.pop();
		}
	}

	public async getMessages(idFrom: Message | undefined, dir: ListFetchDir): Promise<FetchResult<Message>> {
		const selected = get(this.selectedChat);
		if (selected === undefined) return Chat.EmptyFetch;
		let public_key = selected.connection.book.server.publicKey;
		if (public_key === undefined) {
			console.error("Cannot get messages for a non-existant connection");
			return Chat.EmptyFetch;
		}

		let startTime;
		let startId;
		let loadAtBeginning: boolean | undefined;
		switch (dir) {
			case ListFetchDir.Before: loadAtBeginning = true; break;
			case ListFetchDir.New: loadAtBeginning = undefined; break;
			case ListFetchDir.After: loadAtBeginning = false; break;
			default: assert(false, "Unknown direction");
		}

		if (idFrom) {
			startTime = idFrom.date.unix();
			startId = idFrom.id;
		}

		const res = await graphql(`query GetMessages($chatType: GMessageTarget!, $server: [Int!]!, $chatId: ID,
					$startTime: NaiveDateTime, $startId: ID, $loadAtBeginning: Boolean) {
				chat(typ: $chatType, server: $server, id: $chatId) {
					lastRead
					timezone
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
						time
						timezone
					}
				}
			}`, {
			chatType: selected.node.qlType,
			server: public_key,
			chatId: selected.node.qlId,
			startTime,
			startId,
			loadAtBeginning,
		});
		if ("data" in res) {
			// We never chatted here
			if (!res.data.chat || res.data.chat.messages.length === 0) {
				console.log("No chats here");
				return Chat.EmptyFetch;
			}

			const msgs: Message[] = [];
			res.data.chat.messages.forEach((msg: any) => {
				let client;
				if (msg.invoker) {
					client = GraphQlClient.fromGraphqlInvoker(msg.invoker);
				}
				msgs.push(new Message(msg.id, client, msg.invokerName,
					msg.content, msg.rendered, datetimeDeserialize([msg.time, msg.timezone])));
			});
			console.log("Fetching messages " + (loadAtBeginning ? "before" : "after"), [startTime, startId], "; got", msgs);

			Chat.groupMessages(msgs, idFrom, dir);

			return {
				items: msgs,
				canLoadBeforeStart: true,
				canLoadAfterEnd: dir !== ListFetchDir.New // Heuristic: when fetching new we start at the end
			};
		} else {
			console.error("GetMessages result does not contain data", res);
			return Chat.EmptyFetch;
		}
	}

	public sendMessage(message: string) {
		const selected = get(this.selectedChat);
		if (selected === undefined) return;
		selected.connection.sendMessage({
			SendMessage: {
				target: selected.node.wsTarget,
				message,
			}
		});
	}

	public async setLastRead(messageId: string, lastRead: Moment) {
		const selected = get(this.selectedChat);
		if (selected === undefined) return;
		let public_key = selected.connection.book.server.publicKey;
		if (public_key === undefined) {
			console.error("Cannot get messages for a non-existant connection");
			return;
		}
		const res = await graphql(`mutation SetLastRead($chatType: GMessageTarget!, $server: [Int!]!, $chatId: ID,
					$message: ID!) {
				setLastRead(typ: $chatType, server: $server, id: $chatId, message: $message)
			}`, {
			chatType: selected.node.qlType,
			server: public_key,
			chatId: selected.node.qlId,
			message: messageId,
		});
		selected.node.updateChat(new ChatData(lastRead, res.data.setLastRead));
	}
}

export class Message {
	private readonly _clientColor: Lazy<string>;
	public displayDateSeparator: boolean = false;
	public displayGroupHeader: boolean = false;

	public get displayName(): string { return this.invoker?.name ?? this.invokerName ?? ""; }
	public get clientColor() { return this._clientColor.get(); }

	constructor(
		public id: string,
		public invoker: GraphQlClient | undefined,
		public invokerName: string | undefined,
		public raw: string,
		public rendered: string,
		public date: Moment,
	) {
		this._clientColor = new Lazy(() => this.generateClientColor());
	}

	private generateClientColor(): string {
		if (this.invoker?.uid) {
			return getDataColor(this.invoker.uid)
		} else {
			return getDataColor(this.displayName);
		}
	}

	public hasSameInvoker(other: Message): boolean { return Message.hasSameInvoker(this, other); }

	// TODO recheck
	public static hasSameInvoker(first: Message, second: Message): boolean {
		if (first.invoker === second.invoker) return true;
		if (first.invoker === undefined || second.invoker === undefined) {
			return first.invokerName === second.invokerName;
		}
		return first.invoker.equals(second.invoker);
	}
}
