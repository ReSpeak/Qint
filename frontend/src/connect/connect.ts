import { Connection } from "../connection";

export default class Self {
	public username: string = "";
	public address: string = "";
	public bookmark: number | undefined;
	public channelId: number | undefined;

	constructor(
		private connection: Connection
	) { }

	public connect() {
		let version;
		let platform = ((window.navigator as any).oscpu ?? window.navigator.userAgent).toLowerCase();
		if (platform.includes("windows")) {
			version = "Windows_3_X_X__1";
		} else if (platform.includes("linux")) {
			version = "Linux_3_X_X";
		} else if (platform.includes("android")) {
			version = "Android_3_X_X";
		} else if (platform.includes("ios")) {
			version = "iOS_3_X_X";
		} else if (platform.includes("mac")) {
			version = "OS_X_3_X_X";
		} else {
			version = "Windows_3_X_X__2";
		}

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
				version,
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
