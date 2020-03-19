import { writable, Writable, derived, Readable } from "svelte/store";
import moment from "moment";
import { Moment } from "moment";

export class Chat {
	public selected_chat?: string;

	public messages: Writable<Message[]> = writable([]);

	public grouped_messages: Readable<GroupedMessages[]>
		= derived(this.messages, Chat.group_messages);

	private static group_messages(messages: Message[]): GroupedMessages[] {
		let groups = [];
		let current_group: GroupedMessages | undefined;
		for (const message of messages) {
			if(!current_group || message.user !== current_group.user) {
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

export class GroupedMessages {
	constructor(
		public user: string,
		public messages: Message[] = [],
	) { }
}
