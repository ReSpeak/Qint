// tslint:disable: interface-name

import { Channel, ChannelGroup, Client, Server, ServerGroup } from "../book";
import { ChannelId, ChannelGroupId, ClientId, ServerGroupId, OffsetDateTime } from "../ts";
import { Error, InMessage, OChange, Reason, Version } from "../book_events";

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
		bookmark: string | undefined;
		address: string;
		name: string;
		channel: string | undefined;
		version: Version;
		ignoreIdentityMismatch: boolean;
		logCommands: boolean;
		logPackets: boolean;
		logUdpPackets: boolean;
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
		returnCode?: string;
	};
}

interface OMsgSendCommand {
	SendCommand: {
		command: string;
		returnCode?: string;
	};
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
	Change: {
		change: OChange;
		returnCode?: string;
	};
}


// In Messages
export type InMsg = InMsgConnected | InDisconnectedTemporarily | InDisconnected | InMsgError | InTalkersChanged | InMsgEvents | InMsgMessage | InLoudness | InResult;

interface InMsgConnected {
	Connected: {
		server: number[];
		ownClient: ClientId,
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
	Error: Error;
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

export interface ResultDetails {
	tsResult?: string;
	description?: string;
}

export interface InResult {
	Result: {
		returnCode: string;
	} & ResultDetails;
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
