import { Moment } from "moment";

export type ChannelId = string;
export type ChannelGroupId = string;
export type ClientDbId = string;
export type ClientId = string;
export type IconId = string;
export type IpAddr = string;
export type ServerGroupId = string;
export type SocketAddr = string;
export type Uid = number[];

export type EccKeyPubP256 = number[];

export type ClientType = "Normal" | ClientTypeQuery;
interface ClientTypeQuery {
	Query: {
		admin: boolean;
	};
}

export interface TalkPowerRequest {
	time: Moment;
	message: string;
}

export enum TalkState {
	Off,
	Voice,
	Whisper
}

export enum MaxClientsMode {
	Inherited = "Inherited",
	Unlimited = "Unlimited",
	Limited = "Limited",
}

export type MaxClients = "Inherited" | "Unlimited" | { Limited: number };
export type OffsetDateTime = [number, number];
export type RustDuration = [number, number];