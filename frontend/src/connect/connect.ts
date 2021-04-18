import { assert, getDefaultVersion } from "../util";
import { OMsgConnect } from "../backend/ws";
import { ChannelId } from "../ts";

const DEFAULT_NAME: string = "QintUser";

export const enum MuteState {
	None = "None",
	Muted = "Muted",
	Disabled = "Disabled",
}

export interface MuteStates {
	input: MuteState,
	output: MuteState,
	away: boolean,
}

export class ConnectData {

	constructor(
		public name: string,
		public address: string,
		public bookmark?: string,
		public identityId?: string,
		public channel?: string,
		public channelId?: ChannelId,
		public inputMuted?: MuteState,
		public outputMuted?: MuteState,
		public away?: string,
		public password?: string,
		public channelPassword?: string) { }

	public clone(): ConnectData {
		return new ConnectData(this.name, this.address, this.bookmark,
			this.identityId, this.channel, this.channelId, this.inputMuted,
			this.outputMuted, this.away, this.password, this.channelPassword);
	}

	public toConnectMsg(): OMsgConnect {
		const channel = this.channelId !== undefined ? "/" + this.channelId : this.channel;
		return {
			Connect: {
				bookmark: this.bookmark,
				identityId: this.identityId,
				address: this.address,
				name: this.name,
				channel,
				version: getDefaultVersion(),
				inputMuted: this.inputMuted === MuteState.Muted ? true : undefined,
				inputHardwareEnabled: this.inputMuted === MuteState.Disabled ? false : undefined,
				outputMuted: this.outputMuted === MuteState.Muted ? true : undefined,
				outputHardwareEnabled: this.outputMuted === MuteState.Disabled ? false : undefined,
				away: this.away,
				password: this.password,
				channelPassword: this.channelPassword,
				ignoreIdentityMismatch: false,
				logCommands: false,
				logPackets: false,
				logUdpPackets: false,
			}
		};
	}

	public static fromJSON(data: string | any): ConnectData {
		if (typeof data !== "string") {
			assert("address" in data, "Connection needs an address");
			assert("address" in data, "connection data needs an address");
			if (!("name" in data))
				data.name = DEFAULT_NAME;
			return new ConnectData(data.name, data.address, data.bookmark,
				data.identityId, data.channel, data.channelId, data.inputMuted,
				data.outputMuted, data.away, data.password, data.channelPassword);
		} else {
			let start = data.indexOf("@");
			const name = start === -1 ? DEFAULT_NAME : data.substr(0, start);
			start += 1;
			const end = data.indexOf("/");
			const channel = end === -1 ? "" : data.substr(end + 1);
			const address = data.substr(start, end === -1 ? undefined : end);
			return new ConnectData(name, address, undefined, undefined, channel);
		}
	}
}
