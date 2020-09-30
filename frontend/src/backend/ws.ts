// tslint:disable: interface-name

import { Channel, ChannelGroup, Client, Server, ServerGroup } from "../book";
import { ChannelId, ChannelGroupId, ChannelType, ClientId, Codec, ServerGroupId } from "../ts";

export type WsMessageTarget =
	"Server"
	| "Channel"
	| { Client: number }
	| { Poke: number };

// Out Messages
export type OutMsg = OMsgConnect | OMsgDisconnect | OMsgSendMessage | OMsgSetLoudnessThreshold | OMsgSubscribeLoudness | OMsgChange;

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

export enum Reason {
	None = "None",
	Moved = "Moved",
	Subscription = "Subscription",
	LostConnection = "LostConnection",
	KickChannel = "KickChannel",
	KickServer = "KickServer",
	KickServerBan = "KickServerBan",
	Serverstop = "Serverstop",
	Clientdisconnect = "Clientdisconnect",
	Channelupdate = "Channelupdate",
	Channeledit = "Channeledit",
	ClientdisconnectServerShutdown = "ClientdisconnectServerShutdown",
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

interface OMsgSetLoudnessThreshold {
	SetLoudnessThreshold: number;
}

interface OMsgSubscribeLoudness {
	SubscribeLoudness: boolean;
}

interface OMsgChange {
	Change: ChangeChannelEdit | ChangeChannelMove | ChangeClientEdit | ChangeClientUpdate
	| ChangeClientMove | ChangeClientAddServerGroup | ChangeClientRemoveServerGroup
	| ChangeClientKick;
}

interface ChangeChannelEdit {
	ChannelEdit: {
		id: ChannelId,
		password?: string | null;
		channel_type?: ChannelType;
		description?: string;
		order?: ChannelId;
		name?: string;
		topic?: string;
		is_default?: boolean;
		codec?: Codec;
		codec_quality?: number;
		needed_talk_power?: number;
		icon?: number;
		codec_latency_factor?: number;
		is_unencrypted?: boolean;
		delete_delay?: any;
		phonetic_name?: string;
	};
}

interface ChangeChannelMove {
	ChannelMove: {
		id: ChannelId;
		parent: ChannelId;
		order: ChannelId;
	};
}

interface ChangeClientEdit {
	ClientEdit: {
		id: ClientId,
		description?: string;
		talk_power_granted?: boolean;
	};
}

interface ChangeClientUpdate {
	ClientUpdate: {
		name?: string;
		input_muted?: boolean;
		output_muted?: boolean;
		away?: string | null;
	}
}

interface ChangeClientMove {
	ClientMove: {
		id: ClientId,
		channel: ChannelId,
		password?: string;
	}
}

interface ChangeClientAddServerGroup {
	ClientAddServerGroup: {
		id: ClientId,
		server_group: ServerGroupId,
	}
}

interface ChangeClientRemoveServerGroup {
	ClientRemoveServerGroup: {
		id: ClientId,
		server_group: ServerGroupId,
	}
}

interface ChangeClientKick {
	ClientKick: {
		id: ClientId,
		reason: Reason,
		reason_message?: string,
	}
}


// In Messages
export type InMsg = InMsgConnected | InDisconnectedTemporarily | InDisconnected | InMsgError | InTalkersChanged | InMsgEvents | InLoudness;

interface InMsgConnected {
	Connected: {
		server: string;
		own_client: number,
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
	TalkersChanged: [number, boolean][];
}

interface InMsgEvents {
	Events: InBookMsg[];
}

interface InLoudness {
	Loudness: number;
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

type IMskBookChannelListFinished = "ChannelListFinished";

interface IMsgBookMessage {
	Message: {
		target: WsMessageTarget;
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
