export type ChatTarget = IChatTypeServer | IChatTypeChannel | IChatTypeClient;

export enum ChatType {
	Server,
	Channel,
	Client,
}

const server: IChatTypeServer = { type: ChatType.Server };
export const ChatTarget = {
	ToServer(): IChatTypeServer {
		return server;
	},
	ToChannel(id: number): IChatTypeChannel {
		return { type: ChatType.Channel, id };
	},
	ToClient(id: number): IChatTypeClient {
		return { type: ChatType.Client, id };
	},
};

interface IChatTypeServer {
	readonly type: ChatType.Server;
}

interface IChatTypeChannel {
	readonly type: ChatType.Channel;
	id: number;
}

interface IChatTypeClient {
	readonly type: ChatType.Client;
	id: number;
}
