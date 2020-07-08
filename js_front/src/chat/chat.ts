import { get, writable, Writable, derived, Readable } from "svelte/store";
import moment from "moment";
import { Moment } from "moment";
import { Connection } from "../connection";
import { graphql, toDatetime } from "../graphql";
import { MessageTarget } from "../structs/ts";
import { Channel, Client, GraphQlClient } from "../tree/book";
import { getDataColor, assert, Lazy } from "../util";
import { ListFetchDir, FetchResult } from "../ui/lazyList";

export class Chat {
	public readonly selectedChat: Writable<MessageTarget> = writable(MessageTarget.ToServer());
	public readonly unreadCount: Writable<number> = writable(0);
	public composing: string = "";
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
		this.composing = "";
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
		if (lastEntry) {
			lastEntry.displayGroupHeader = false;
			lastEntry.displayDateSeparator = false;
			if (dir === ListFetchDir.Before) messages.unshift(lastEntry);
			else if (dir === ListFetchDir.After) messages.push(lastEntry);
		}

		let previousMessage: Message | undefined;
		let previousDate: Moment | undefined;
		for (const message of messages) {
			message.displayGroupHeader = !previousMessage || !GraphQlClient.equals(previousMessage.invoker, message.invoker);
			message.displayDateSeparator = !previousDate || !previousDate.isSame(message.date, "day");
			previousMessage = message;
			previousDate = message.date;
		}

		if (lastEntry) {
			if (dir === ListFetchDir.Before) messages.shift();
			else if (dir === ListFetchDir.After) messages.pop();
		}
	}

	public async getMessages(idFrom: Message | undefined, dir: ListFetchDir): Promise<FetchResult<Message>> {
		if (this.connection.server === undefined) {
			console.error("Cannot get messages for a non-existant connection");
			return Chat.EmptyFetch;
		}

		let start_time;
		let start_id;
		let before_start: boolean | undefined;
		switch (dir) {
			case ListFetchDir.Before: before_start = true; break;
			case ListFetchDir.New: before_start = undefined; break;
			case ListFetchDir.After: before_start = false; break;
			default: assert(false, "Unknown direction");
		}

		if (idFrom) {
			start_time = idFrom.date.unix();
			start_id = idFrom.id;
		}

		const res = await graphql(`query GetMessages($chat_type: GMessageTarget!, $server: ID!, $chat_id: ID,
					$start_time: NaiveDateTime, $start_id: ID, $before_start: Boolean) {
				chat(typ: $chat_type, server: $server, id: $chat_id) {
					lastRead
					timezone
					messages(startTime: $start_time, startId: $start_id, beforeStart: $before_start) {
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
			before_start: before_start,
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
					msg.content, msg.rendered, toDatetime(msg.time, msg.timezone)));
			});
			console.log("Fetching messages " + (before_start ? "before" : "after"), [start_time, start_id], "; got", msgs);

			Chat.groupMessages(msgs, idFrom, dir);

			return {
				items: msgs,
				canLoadBeforeStart: true,
				canLoadAfterEnd: true
			};
		} else {
			console.error("GetMessages result does not contain data", res);
			return Chat.EmptyFetch;
		}
	}

	public sendMessage() {
		const target = get(this.selectedChat);
		if ("Channel" in target)
			target.Channel = null;

		this.connection.sendMessage({
			SendMessage: {
				target,
				message: this.composing
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
