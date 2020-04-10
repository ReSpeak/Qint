import { get, writable, Writable, derived, Readable } from "svelte/store";
import moment from "moment";
import { Moment } from "moment";
import { Connection } from "../connection";
import { MessageTarget } from "../structs/ts";

export class Chat {
	public readonly selectedChat: Writable<MessageTarget> = writable(MessageTarget.ToServer());

	public readonly messages: Writable<Message[]> = writable([]);

	public composing: string = "";

	public readonly groupedMessages: Readable<ChatEntries[]>
		= derived(this.messages, Chat.group_messages);

	constructor(
		private connection: Connection
	) { }

	private static group_messages(messages: Message[]): ChatEntries[] {
		const groups = [];
		let currentGroup: GroupedMessages | undefined;
		let currentDate: Moment | undefined;
		for (const message of messages) {
			if (!currentGroup || message.user !== currentGroup.user) {
				if (!currentDate || !currentDate.isSame(message.date, "day") ) {
					currentDate = message.date;
					groups.push(new DateSeparator(message.date));
				}
				currentGroup = new GroupedMessages(message.user);
				groups.push(currentGroup);
			}
			currentGroup.messages.push(message);
		}
		return groups;
	}

	public sendMessage() {
		this.connection.sendMessage({
			SendMessage: {
				target: get(this.selectedChat),
				message: this.composing
			}
		});
	}
}

export class Message {
	constructor(
		public user: string,
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
		public user: string
	) { }
}
