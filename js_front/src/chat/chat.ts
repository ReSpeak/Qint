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
	public static readonly EmptyFetch: FetchResult<GroupedMessages> = {
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

	private static tryMergeGroups(a: GroupedMessages, b: GroupedMessages, intoFirst: boolean): boolean {
		if (!GraphQlClient.equals(a.invoker, b.invoker))
			return false;
		const into = intoFirst ? a : b;
		into.messages = [...a.messages, ...b.messages];
		return true;
	}

	private static groupMessages(messages: Message[], lastEntry: GroupedMessages | undefined, dir: ListFetchDir): GroupedMessages[] {
		const groups = [];
		let currentGroup: GroupedMessages | undefined;
		let currentDate: Moment | undefined;

		for (const message of messages) {
			if (!currentGroup || !GraphQlClient.equals(currentGroup?.invoker, message.invoker)) {
				currentGroup = new GroupedMessages();
				groups.push(currentGroup);
				if (!currentDate || !currentDate.isSame(message.date, "day")) {
					currentDate = message.date;
					currentGroup.displayDateSeparator = true;
				}
			}
			currentGroup.messages.push(message);
		}

		if (groups.length > 0 && lastEntry !== undefined) {
			if (dir === ListFetchDir.Before) {
				if (this.tryMergeGroups(groups[groups.length - 1], lastEntry, false))
					groups.length -= 1; // remove last (the one merged into lastEntry)
			} else {
				if (this.tryMergeGroups(lastEntry, groups[0], true))
					groups.shift(); // remove first (the one merged into lastEntry)
			}
		}

		return groups;
	}

	public async getMessages(idFrom: GroupedMessages | undefined, dir: ListFetchDir): Promise<FetchResult<GroupedMessages>> {
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
			let message;
			if (before_start) message = idFrom.getFirstMessage();
			else message = idFrom.getLastMessage();

			start_time = message.date.unix();
			start_id = message.id;
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

			return {
				items: Chat.groupMessages(msgs, idFrom, dir),
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
	constructor(
		public id: string,
		public invoker: GraphQlClient | undefined,
		public invokerName: string | undefined,
		public raw: string,
		public rendered: string,
		public date: Moment,
	) { }

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

export class GroupedMessages {
	private _clientColor: Lazy<string>;
	public displayDateSeparator: boolean = false;
	public messages: Message[] = [];
	public get clientColor() { return this._clientColor.get(); }
	public get invoker() { return this.getFirstMessage().invoker; }
	public get invokerName() { return this.getFirstMessage().invokerName; }
	public get topDate() { return this.getFirstMessage().date; }
	public get displayName() {
		const msg = this.getFirstMessage();
		return msg.invoker?.name ?? msg.invokerName ?? "";
	}

	constructor() {
		this._clientColor = new Lazy(() => this.generateClientColor());
	}

	public getFirstMessage(): Message {
		assert(this.messages.length > 0, "Why the hell is an empty group here?");
		return this.messages[0];
	}

	public getLastMessage(): Message {
		assert(this.messages.length > 0, "Why the hell is an empty group here?");
		return this.messages[this.messages.length - 1];
	}

	private generateClientColor(): string {
		const msg = this.getFirstMessage();
		if (msg.invoker?.uid) {
			return getDataColor(msg.invoker.uid)
		} else {
			return getDataColor(this.displayName);
		}
	}
}
