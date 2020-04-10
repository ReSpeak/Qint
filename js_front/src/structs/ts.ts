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
};