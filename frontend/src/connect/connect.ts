import { getDefaultVersion } from "../util";
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
}
