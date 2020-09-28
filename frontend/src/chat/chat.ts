import { get, writable, Writable } from "svelte/store";
import type { Moment } from "moment";
import { Connection } from "../connection";
import { graphql } from "../graphql";
import { MessageTarget } from "../ts";
import { Channel, Client, GraphQlClient } from "../book";
import { datetimeDeserialize, getDataColor, assert, Lazy } from "../util";
import { ListFetchDir, FetchResult } from "../ui/lazyList";

export class Chat {
	public readonly selectedChat: Writable<MessageTarget> = writable(MessageTarget.ToServer());
	public readonly unreadCount: Writable<number> = writable(0);
	public static readonly EmptyFetch: FetchResult<Message> = {
		items: [],
		canLoadBeforeStart: false,
		canLoadAfterEnd: false
	};

	constructor(
		private connection: Connection
	) { }

	public reset() {
		this.selectedChat.set(MessageTarget.ToServer());
		this.unreadCount.set(0);
	}

	public selectChannel(channel: Channel) {
		this.selectedChat.set(MessageTarget.ToChannel(channel.id));
	}

	public selectClient(client: Client) {
		this.selectedChat.set(MessageTarget.ToClient(client.id));
	}

	public selectServer() {
		this.selectedChat.set(MessageTarget.ToServer());
	}

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
		if (this.connection.server === undefined) {
			console.error("Cannot get messages for a non-existant connection");
			return Chat.EmptyFetch;
		}

		let start_time;
		let start_id;
		let load_at_beginning: boolean | undefined;
		switch (dir) {
			case ListFetchDir.Before: load_at_beginning = true; break;
			case ListFetchDir.New: load_at_beginning = undefined; break;
			case ListFetchDir.After: load_at_beginning = false; break;
			default: assert(false, "Unknown direction");
		}

		if (idFrom) {
			start_time = idFrom.date.unix();
			start_id = idFrom.id;
		}

		const res = await graphql(`query GetMessages($chat_type: GMessageTarget!, $server: ID!, $chat_id: ID,
					$start_time: NaiveDateTime, $start_id: ID, $load_at_beginning: Boolean) {
				chat(typ: $chat_type, server: $server, id: $chat_id) {
					lastRead
					timezone
					messages(startTime: $start_time, startId: $start_id, beforeStart: $load_at_beginning) {
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
			chat_type: MessageTarget.getType(get(this.selectedChat)),
			server: this.connection.server,
			chat_id: MessageTarget.getId(get(this.selectedChat), this.connection),
			start_time,
			start_id,
			load_at_beginning: load_at_beginning,
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
			console.log("Fetching messages " + (load_at_beginning ? "before" : "after"), [start_time, start_id], "; got", msgs);

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
		const target = MessageTarget.toWs(get(this.selectedChat));
		this.connection.sendMessage({
			SendMessage: {
				target,
				message,
			}
		});
	}
}

export class Message {
	private _clientColor: Lazy<string>;
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
