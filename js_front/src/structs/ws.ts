import { Client, Channel, Server } from "../tree/book";
// tslint:disable: interface-name

// Out Messages
export type OutMsg =
	{ Connect: OMsgConnect }
	| { SendMessage: OMsgSendMessage };

interface OMsgConnect {
	address: string;
	name: string;
	log_commands: boolean;
	log_packets: boolean;
	log_udp_packets: boolean;
	version: string;
}

interface OMsgSendMessage {
	target: string; // TODO
	message: string;
}

// In Messages
export type InMsg = InMsgError | InTalkersChanged | InMsgEvents;

interface InMsgError {
	Error: string;
}

interface InTalkersChanged {
	TalkersChanged: [number, boolean][];
}

interface InMsgEvents {
	Events: InBookMsg[];
}

// export type IMsgBookAdd = IMsgBookAddClient | IMsgBookAddChannel | IMsgBookAddServer;
// export type IMsgBookChange = IMsgBookChangeClient | IMsgBookChangeChannel | IMsgBookChangeServer;
// export type IMsgBookRemove = IMsgBookRemoveClient | IMsgBookRemoveChannel;

// type InBookMsg = IMsgBookAdd | IMsgBookChange | IMsgBookRemove | IMsgConnected;

type InBookMsg = any;

// type BookOp<TOp extends string, TTo extends string, TObj> = {
// 	to: TTo;
// 	obj: TObj;
// } & IMsg<TOp>;

// type SAdd = "b_add";
// type SChange = "b_change";
// type SRemove = "b_remove";
// type SClient = "client";
// type SChannel = "channel";
// type SServer = "server";

// type IMsgBookAddClient = BookOp<SAdd, SClient, Client>;
// type IMsgBookAddChannel = BookOp<SAdd, SChannel, Channel>;
// type IMsgBookAddServer = BookOp<SAdd, SServer, Server>;
// type IMsgBookChangeClient = BookOp<SChange, SClient, Partial<Client>>;
// type IMsgBookChangeChannel = BookOp<SChange, SChannel, Partial<Channel>>;
// type IMsgBookChangeServer = BookOp<SChange, SServer, Partial<Server>>;
// type IMsgBookRemoveClient = BookOp<SRemove, SClient, { id: number }>;
// type IMsgBookRemoveChannel = BookOp<SRemove, SChannel, { id: number }>;

// tslint:disable-next-line: no-empty-interface
