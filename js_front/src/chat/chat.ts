import { writable, Writable, derived, Readable } from "svelte/store";
import moment from "moment";
import { Moment } from "moment";
import { ChatTarget } from "../structs/ts";

export class Chat {
	public readonly selected_chat: Writable<ChatTarget> = writable(ChatTarget.ToServer());

	public readonly messages: Writable<Message[]> = writable([]);

	public readonly grouped_messages: Readable<ChatEntries[]>
		= derived(this.messages, Chat.group_messages);

	private static group_messages(messages: Message[]): ChatEntries[] {
		const groups = [];
		let current_group: GroupedMessages | undefined;
		let current_date: Moment | undefined;
		for (const message of messages) {
			if (!current_group || message.user !== current_group.user) {
				if (!current_date || !current_date.isSame(message.date, "day") ) {
					current_date = message.date;
					groups.push(new DateSeparator(message.date));
				}
				current_group = new GroupedMessages(message.user);
				groups.push(current_group);
			}
			current_group.messages.push(message);
		}
		return groups;
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
