import { Connection } from "./connection";

export default class Self {
	public username: string = "";
	public address: string = "";

	constructor(
		private connection: Connection
	) { }

	public connect() {
		this.connection.connect({
			address: this.address,
			name: this.username
		});
	}

	public reset() {
		this.connection.reset();
	}
}
