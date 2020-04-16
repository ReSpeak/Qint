export type MessageTarget =
	{ Server: null}
	| { Channel: number }
	| { Client: number }
	| { Poke: number };

export const MessageTarget = {
	ToServer(): MessageTarget {
		return { Server: null };
	},
	ToChannel(id: number): MessageTarget {
		return { Channel: id };
	},
	ToClient(id: number): MessageTarget {
		return { Client: id };
	},
	ToClientPoke(id: number): MessageTarget {
		return { Poke: id };
	},

	getType(target: MessageTarget): string {
		if ("Server" in target) {
			return "SERVER";
		} else if ("Channel" in target) {
			return "CHANNEL";
		} else if ("Client" in target) {
			return "CLIENT";
		} else if ("Poke" in target) {
			return "POKE";
		} else {
			throw "Invalid message target type";
		}
	},

	getId(target: MessageTarget): string | undefined {
		if ("Channel" in target) {
			return target.Channel.toString();
		} else if ("Client" in target) {
			return target.Client.toString();
		} else if ("Poke" in target) {
			return target.Poke.toString();
		}
		return;
	}
};