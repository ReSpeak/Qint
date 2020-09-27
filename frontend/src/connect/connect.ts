import { Connection } from "../connection";
import { getDefaultVersion } from "../util";

export default class Self {
	public username: string = "";
	public address: string = "";
	public bookmark: number | undefined;
	public channelId: number | undefined;

	constructor(
		private connection: Connection
	) { }

	public connect() {
		const sep = this.address.indexOf("/");
		let address = this.address;
		let channel = this.channelId === undefined ? undefined : "/" + this.channelId;
		if (channel === undefined && sep !== -1) {
			address = this.address.slice(0, sep);
			channel = this.address.slice(sep + 1);
		}
		this.connection.connect({
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
		});
	}

	public reset() {
		this.connection.reset();
	}
}
