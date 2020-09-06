import { get } from "svelte/store";
import { Connection } from "../connection";
import { WsMessageTarget } from "./ws";

export type ChannelId = number;
export type ChannelGroupId = number;
export type ClientId = number;
export type ServerGroupId = number;

export type MessageTarget =
	{ Server: null }
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

	toWs(target: MessageTarget): WsMessageTarget {
		if ("Server" in target) {
			return "Server";
		} else if ("Channel" in target) {
			return "Channel";
		} else if ("Client" in target) {
			return { Client: target.Client };
		} else if ("Poke" in target) {
			return { Poke: target.Poke };
		} else {
			throw "Invalid message target type";
		}
	},

	toUniqueString(target: MessageTarget, con: Connection): string | undefined {
		if ("Server" in target) {
			return `SERVER,${con.server}`;
		} else if ("Channel" in target) {
			return `CHANNEL,${con.server},${target.Channel}`;
		} else if ("Client" in target) {
			const uid = get(con.book.clients).get(target.Client)?.uidStr;
			if (uid === undefined) return undefined;
			return `CLIENT,${uid}`;
		} else if ("Poke" in target) {
			return `POKE,${target.Poke}`;
		} else {
			throw "Invalid message target type";
		}
	},

	getId(target: MessageTarget, connection: Connection): string | undefined {
		if ("Channel" in target) {
			return target.Channel.toString();
		} else if ("Client" in target) {
			// TODO Should be uid
			const uid = get(connection.book.clients).get(target.Client)?.uid ?? [];
			let uidStr = "";
			uid.forEach((c: number) => {
				uidStr += String.fromCharCode(c);
			});
			return btoa(uidStr);
		} else if ("Poke" in target) {
			return target.Poke.toString();
		}
		return;
	}
};

export enum Codec {
	SpeexNarrowband = "SpeexNarrowband",
	SpeexWideband = "SpeexWideband",
	SpeexUltrawideband = "SpeexUltrawideband",
	CeltMono = "CeltMono",
	OpusVoice = "OpusVoice",
	OpusMusic = "OpusMusic",
}

export function codecToName(codec: Codec) {
	switch (codec) {
		case Codec.SpeexNarrowband: return "Speex Narrowband";
		case Codec.SpeexWideband: return "Speex Wideband";
		case Codec.SpeexUltrawideband: return "Speex Ultrawideband";
		case Codec.CeltMono: return "Celt Mono";
		case Codec.OpusVoice: return "Opus Voice";
		case Codec.OpusMusic: return "Opus Music";
		default: return "Unknown";
	}
}

export enum ChannelType {
	Permanent = "Permanent",
	SemiPermanent = "SemiPermanent",
	Temporary = "Temporary"
}
