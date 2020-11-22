import { assert, getDefaultVersion } from "../util";
import { OMsgConnect } from "../backend/ws";
import { ChannelId } from "../ts";

export class ConnectData {
	constructor(
		public name: string,
		public address: string,
		public bookmark?: string,
		public channel?: string,
		public channelId?: ChannelId,
		public inputMuted?: boolean,
		public outputMuted?: boolean,
		public away?: string) { }

	public clone(): ConnectData {
		return new ConnectData(this.name, this.address, this.bookmark,
			this.channel, this.channelId, this.inputMuted,
			this.outputMuted, this.away);
	}

	public toConnectMsg(): OMsgConnect {
		const channel = this.channelId !== undefined ? "/" + this.channelId : this.channel;
		return {
			Connect: {
				bookmark: this.bookmark,
				address: this.address,
				name: this.name,
				channel,
				version: getDefaultVersion(),
				inputMuted: this.inputMuted,
				outputMuted: this.outputMuted,
				away: this.away,
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
				data.name = "TeamSpeakUser";
			return new ConnectData(data.name, data.address, data.bookmark,
				data.channel, data.channelId, data.inputMuted, data.outputMuted,
				data.away);
		} else {
			let start = data.indexOf("@");
			let name = start === -1 ? "TeamSpeakUser" : data.substr(0, start);
			start += 1;
			let end = data.indexOf("/");
			let channel = end === -1 ? "" : data.substr(end + 1);
			let address = data.substr(start, end === -1 ? undefined : end);
			return new ConnectData(name, address, undefined, channel);
		}
	}

	public toJSON(): string | ConnectData {
		if (this.bookmark === undefined &&
			this.inputMuted === undefined &&
			this.outputMuted === undefined &&
			this.away === undefined) {
			let s = "";
			if (this.name !== "TeamSpeakUser")
				s = this.name + "@";
			s += this.address;
			if (this.channel !== undefined)
				s += "/" + this.channel;
			else if (this.channelId !== undefined)
				s += "//" + this.channelId;
			return s;
		} else {
			return this;
		}
	}
}
