import { get, writable, Writable, derived, Readable } from "svelte/store";
import moment from "moment";
import { Moment } from "moment";
import { Connection } from "../connection";
import { graphql, toDatetime } from "../graphql";
import { MessageTarget } from "../structs/ts";
import { Channel, Client, GraphQlClient } from "../tree/book";
import { getDataColor, arraysEqual } from "../util";

export class Chat {
	public readonly selectedChat: Writable<MessageTarget> = writable(MessageTarget.ToServer());
	public readonly unreadCount: Writable<number> = writable(0);
	public composing: string = "";

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

	private static isSameInvoker(group: GroupedMessages, msg: Message): boolean {
		if (group.invoker) {
			if (!msg.invoker)
				return false;
			if (!arraysEqual(group.invoker.uid, msg.invoker.uid))
				return false;
		}
		return group.invokerName === msg.invokerName;
	}

	private static group_messages(messages: Message[]): ChatEntries[] {
		const groups = [];
		let currentGroup: GroupedMessages | undefined;
		let currentDate: Moment | undefined;
		for (const message of messages) {
			if (!currentDate || !currentDate.isSame(message.date, "day") ) {
				currentDate = message.date;
				groups.push(new DateSeparator(message.date));
				currentGroup = undefined;
			}
			if (!currentGroup || !Chat.isSameInvoker(currentGroup, message)) {
				currentGroup = new GroupedMessages(message.invoker, message.invokerName);
				groups.push(currentGroup);
			}
			currentGroup.messages.push(message);
		}
		return groups;
	}

	public async getMessages(fromStart: boolean, curMsgs: ChatEntries[]): Promise<ChatEntries[] | undefined> {
		if (this.connection.server !== undefined) {
			let start_time;
			let start_id;
			let lastMsg;

			let i = fromStart ? 0 : curMsgs.length - 1;
			let step = fromStart ? 1 : -1;
			while (i >= 0 && i < curMsgs.length) {
				const group = curMsgs[i];
				if (group instanceof GroupedMessages && group.messages.length > 0) {
					lastMsg = group.messages[fromStart ? 0 : group.messages.length - 1];
					break;
				}
				i += step;
			}

			if (lastMsg) {
				start_time = lastMsg.date.unix();
				start_id = lastMsg.id;
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
				before_start: start_time ? fromStart : undefined,
			});
			if ("data" in res) {
				// We never chatted here
				if (!res.data.chat || res.data.chat.messages.length == 0)
					return;

				const msgs: Message[] = [];
				res.data.chat.messages.forEach((msg: any) => {
					let client;
					if (msg.invoker) {
						client = GraphQlClient.fromGraphqlInvoker(msg.invoker);
					}
					msgs.push(new Message(msg.id, client, msg.invokerName,
						msg.content, toDatetime(msg.time, msg.timezone)));
				});
				const before_start = start_time ? fromStart : true;
				console.log("Fetching messages " + (before_start ? "before" : "after"), [start_time, start_id], "; got", msgs);

				// TODO We need to combine this with the existing messages
				return Chat.group_messages(msgs);
			} else {
				console.error("GetMessages result does not contain data", res);
			}
		} else {
			console.error("Cannot get messages for a non-existant connection");
		}
		return;
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
		public text: string,
		public date: Moment = moment(),
	) { }
}

type ChatEntries = DateSeparator | GroupedMessages;

export class DateSeparator {
	constructor(
		public date: Moment
	) {}
}

export class GroupedMessages {
	public date?: Moment;
	public messages: Message[] = [];
	constructor(
		public invoker: GraphQlClient | undefined,
		public invokerName: string | undefined,
	) { }

	public getClientColor() {
		if (this.invoker && this.invoker.uid) {
			return getDataColor(this.invoker.uid)
		} else {
			return getDataColor(this.getClientName());
		}
	}

	public getClientName() {
		return (this.invoker ? this.invoker.name : undefined) || this.invokerName || "";
	}
}
