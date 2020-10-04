import { getDefaultVersion } from "../util";
import { OMsgConnect } from "../backend/ws";

export class ConnectData {
	constructor(
		public readonly username: string,
		public readonly address: string,
		public readonly bookmark?: number,
		public readonly channelId?: number) { }

	public toConnectMsg(): OMsgConnect {
		const sep = this.address.indexOf("/");
		let address = this.address;
		let channel = this.channelId === undefined ? undefined : "/" + this.channelId;
		if (sep !== -1) {
			address = this.address.slice(0, sep);
			if (channel === undefined)
				channel = this.address.slice(sep + 1);
		}
		return {
			Connect: {
				bookmark: this.bookmark,
				address,
				name: this.username,
				channel,
				version: getDefaultVersion(),
				ignore_identity_mismatch: false,
				log_commands: false,
				log_packets: false,
				log_udp_packets: false,
			}
		};
	}
}
