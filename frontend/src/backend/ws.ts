// tslint:disable: interface-name

import { ClientId } from "../ts";
import { InMessage, OChange, PropertyId, PropertyValue, Reason, Version, TsError } from "../book_events";

export type WsMessageTarget =
	"Server"
	| "Channel"
	| { Client: ClientId }
	| { Poke: ClientId };

// Out Messages
export type OutMsg = OMsgConnect | OMsgDisconnect | OMsgSendMessage | OMsgSendCommand
	| OMsgSetClientVolume | OMsgChange;

export interface OMsgConnect {
	Connect: {
		bookmark: string | undefined;
		address: string;
		name: string;
		identityId?: string;
		channel: string | undefined;
		version: Version;
		inputMuted?: boolean;
		outputMuted?: boolean;
		away?: string;
		password?: string;
		channelPassword?: string;
		ignoreIdentityMismatch: boolean;
		logCommands: boolean;
		logPackets: boolean;
		logUdpPackets: boolean;
		returnCode?: string;
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
export type InMsg = InMsgConnected | InDisconnectedTemporarily | InDisconnected | InMsgError
	| InTalkersChanged | InMsgEvents | InMsgMessage | InLoudnesses | InResult;

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
	Error: TsError;
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

interface InLoudnesses {
	Loudnesses: Record<ClientId, number>;
}

export interface ResultDetails {
	tsResult?: TsError;
	missingPermission?: number;
	description?: string;
}

export interface InResult {
	Result: {
		returnCode: string;
	} & ResultDetails;
}

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
