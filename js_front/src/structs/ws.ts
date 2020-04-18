import { Client, Channel, Server } from "../tree/book";
import { MessageTarget } from "./ts";
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
	target: MessageTarget;
	message: string;
}

// In Messages
export type InMsg = InMsgConnected | InMsgError | InTalkersChanged | InMsgEvents;

interface InMsgConnected {
	Connected: {
		server: string;
	};
}

interface InMsgError {
	Error: string;
}

interface InTalkersChanged {
	TalkersChanged: [number, boolean][];
}

interface InMsgEvents {
	Events: InBookMsg[];
}

interface IMsgPropertyIdChannel {
	Channel: number;
}

interface IMsgPropertyIdClient {
	Client: number;
}

interface IMsgPropertyIdClientServerGroup {
	ClientServerGroup: [number, number];
}

interface IMsgPropertyIdServer {
	Server: {};
}

type PropertyId = IMsgPropertyIdChannel | IMsgPropertyIdClient | IMsgPropertyIdClientServerGroup | IMsgPropertyIdServer;

interface IMsgPropertyValueChannel {
	Channel: any;
}

interface IMsgPropertyValueClient {
	Client: any;
}

interface IMsgPropertyValueServer {
	Server: any;
}

type PropertyValue = IMsgPropertyValueChannel | IMsgPropertyValueClient | IMsgPropertyValueServer;


interface Invoker {
	name: string;
	id: number;
	uid: string | undefined;
}

interface IMsgBookAdded {
	PropertyAdded: {
		id: PropertyId;
		prop: PropertyValue;
		invoker: Invoker;
	};
}

interface IMsgBookChanged {
	PropertyChanged: {
		id: PropertyId;
		prop: PropertyValue;
		invoker: Invoker;
	};
}

interface IMsgBookRemoved {
	PropertyRemoved: {
		id: PropertyId;
		prop: PropertyValue;
		invoker: Invoker;
	};
}

type IMskBookChannelListFinished = "ChannelListFinished";

interface IMsgBookMessage {
	Message: {
		target: MessageTarget;
		invoker: Invoker;
		message: string;
	};
}

export type InBookChangeMsg = IMsgBookAdded | IMsgBookChanged | IMsgBookRemoved;

export type InBookMsg = InBookChangeMsg | IMskBookChannelListFinished | IMsgBookMessage;

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
