import { getDefaultVersion } from "../util";
import { OMsgConnect } from "../backend/ws";
import { ChannelId } from "../ts";

export class ConnectData {
	constructor(
		public name: string,
		public address: string,
		public bookmark?: string,
		public channel?: string,
		public channelId?: ChannelId) { }

	public static fromConString(name: string, address: string, bookmark?: string, channelId?: ChannelId): ConnectData {
		const sep = address.indexOf("/");
		let addr = address;
		let channel = undefined;
		if (sep !== -1) {
			addr = address.slice(0, sep);
			channel = address.slice(sep + 1);
		}
		return new ConnectData(name, addr, bookmark, channel, channelId);
	}

	public clone(): ConnectData {
		return new ConnectData(this.name, this.address, this.bookmark, this.channel, this.channelId);
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
				ignoreIdentityMismatch: false,
				logCommands: false,
				logPackets: false,
				logUdpPackets: false,
			}
		};
	}
}
