// tslint:disable: interface-name

import { Channel, ChannelGroup, Client, Server, ServerGroup } from "../book";
import { ChannelId, ChannelGroupId, ClientId, ServerGroupId, OffsetDateTime } from "../ts";
import { InMessage, OChange, Reason } from "../book_events";

export type WsMessageTarget =
	"Server"
	| "Channel"
	| { Client: ClientId }
	| { Poke: ClientId };

// Out Messages
export type OutMsg = OMsgConnect | OMsgDisconnect | OMsgSendMessage | OMsgSendCommand | OMsgSetLoudnessThreshold
	| OMsgSubscribeLoudness | OMsgSetClientVolume | OMsgChange;

export interface OMsgConnect {
	Connect: {
		bookmark: number | undefined;
		address: string;
		name: string;
		channel: string | undefined;
		version: string;
		ignore_identity_mismatch: boolean;
		log_commands: boolean;
		log_packets: boolean;
		log_udp_packets: boolean;
	};
}

interface OMsgDisconnect {
	Disconnect: {
		reason?: Reason;
		message?: string;
	};
}

interface OMsgSendMessage {
	SendMessage: {
		target: WsMessageTarget,
		message: string;
	};
}

interface OMsgSendCommand {
	SendCommand: string;
}

interface OMsgSetLoudnessThreshold {
	SetLoudnessThreshold: number;
}

interface OMsgSubscribeLoudness {
	SubscribeLoudness: boolean;
}

interface OMsgSetClientVolume {
	SetClientVolume: {
		client: number[],
		volume: number,
	};
}

interface OMsgChange {
	Change: OChange;
}


// In Messages
export type InMsg = InMsgConnected | InDisconnectedTemporarily | InDisconnected | InMsgError | InTalkersChanged | InMsgEvents | InMsgMessage | InLoudness | InChannelFileList;

interface InMsgConnected {
	Connected: {
		server: number[];
		own_client: ClientId,
	};
}

interface InDisconnectedTemporarily {
	DisconnectedTemporarily: null;
}

/** NOTE: Internal event, not sent over the websocket */
interface InDisconnected {
	Disconnected: null;
}

interface InMsgError {
	Error: string;
}

interface InTalkersChanged {
	TalkersChanged: [ClientId, boolean][];
}

interface InMsgEvents {
	Events: InBookMsg[];
}

interface InMsgMessage {
	Message: InMessage;
}

interface InLoudness {
	Loudness: number;
}

export interface InChannelFileList {
	FileList: InChannelFile[];
}

export interface InChannelFile {
	channelId: string;
	path: string;
	name: string;
	size: number;
	lastModified: OffsetDateTime;
	isFile: boolean;
}

//#region PropertyId

interface IMsgPropertyIdChannel {
	Channel: ChannelId;
}

interface IMsgPropertyIdChannelGroup {
	ChannelGroup: ChannelGroupId;
}

interface IMsgPropertyIdClient {
	Client: ClientId;
}

interface IMsgPropertyIdClientServerGroup {
	ClientServerGroup: [ClientId, ServerGroupId];
}

// TODO: This declaration is wrong
// it should be "Server", but then typescript complanins
// that we can't use 'in' on the union any more
interface IMsgPropertyIdServer {
	Server: {};
}

interface IMsgPropertyIdServerIp {
	ServerIp: string;
}

interface IMsgPropertyIdServerGroup {
	ServerGroup: ServerGroupId;
}

type PropertyId =
	IMsgPropertyIdChannel |
	IMsgPropertyIdChannelGroup |
	IMsgPropertyIdClient |
	IMsgPropertyIdClientServerGroup |
	IMsgPropertyIdServer |
	IMsgPropertyIdServerIp |
	IMsgPropertyIdServerGroup;

//#endregion

//#region PropertyValue

interface IMsgPropertyValueChannel {
	Channel: Partial<Channel>;
}

interface IMsgPropertyValueChannelGroup {
	ChannelGroup: Partial<ChannelGroup>;
}

interface IMsgPropertyValueClient {
	Client: Partial<Client>;
}

interface IMsgPropertyValueServer {
	Server: Partial<Server>;
}

interface IMsgPropertyValueIpAddr {
	IpAddr: string;
}

interface IMsgPropertyValueServerGroup {
	ServerGroup: Partial<ServerGroup>;
}

interface IMsgPropertyValueServerGroupId {
	ServerGroupId: ServerGroupId;
}

type PropertyValue =
	IMsgPropertyValueChannel |
	IMsgPropertyValueChannelGroup |
	IMsgPropertyValueClient |
	IMsgPropertyValueServer |
	IMsgPropertyValueIpAddr |
	IMsgPropertyValueServerGroup |
	IMsgPropertyValueServerGroupId;

//#endregion

export interface Invoker {
	name: string;
	id: number;
	uid: string | undefined;
}

export interface ExtraInfo {
	reason: Reason | null;
}

type PropertyMod = {
	id: PropertyId;
	prop: PropertyValue | undefined;
	invoker: Invoker | null;
	extra: ExtraInfo;
};

interface IMsgBookAdded {
	PropertyAdded: PropertyMod;
}

interface IMsgBookChanged {
	PropertyChanged: PropertyMod;
}

interface IMsgBookRemoved {
	PropertyRemoved: PropertyMod;
}

interface IMsgBookMessage {
	Message: {
		target: WsMessageTarget;
		invoker: Invoker;
		message: string;
	};
}

export type InBookChangeMsg = IMsgBookAdded | IMsgBookChanged | IMsgBookRemoved;

export type InBookMsg = InBookChangeMsg | IMsgBookMessage;

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
