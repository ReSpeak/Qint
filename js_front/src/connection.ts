import { Chat, Message } from "./chat/chat";
import { OutMsg, InMsg, Reason } from "./structs/ws";
import { get, writable, Writable } from "svelte/store";
import { Book, Channel, Server } from "./tree/book";

export class Connection {
	public readonly state = writable(ConnectionState.Disconnected);
	public readonly error: Writable<string | undefined> = writable(undefined);

	public readonly book: Book = new Book();
	public readonly chat: Chat = new Chat(this);
	public server?: string;
	public ownClient?: number;
	private socket?: WebSocket;
	public guid?: string;

	constructor() {
		this.fillDummyData();
	}

	public reset() {
		this.state.set(ConnectionState.Disconnected);
		this.socket?.close();
		this.socket = undefined;
	}

	public connect(opt: IConnectOptions) {
		this.error.set(undefined);
		this.guid = Connection.createUuidV4();
		this.socket = new WebSocket(`ws://localhost:4422/con/${this.guid}/ws?format=Json`);
		this.socket.onopen = () => {
			this.sendMessage({
				Connect: {
					address: opt.address,
					name: opt.name,
					log_commands: false,
					log_packets: false,
					log_udp_packets: false,
					version: "Linux_5_0_0_test_87"
				}
			});
		};
		this.socket.onerror = (error) => {
			this.error.set("Connection failed, is Qint running?");
		};
		this.socket.onclose = () => this.reset();
		this.socket.onmessage = (evt) => this.messageHandler(evt);
		this.state.set(ConnectionState.Connecting);
	}

	// See https://jsperf.com/node-uuid-performance/64 about how to generate a uuid fast
	private static createUuidV4(): string {
		var d2h: string[] = [], vals = new Array(16);
		for (var i = 0; i < 256; ++i) d2h.push((0x100 + i).toString(16).substr(1));

		for (var i = 0; i < 16; ++i) vals[i] = Math.random() * 256 | 0;
		vals[6] = vals[6] & 0x0f | 0x40;
		vals[8] = vals[8] & 0x3f | 0x80;
		return d2h[vals[0]] + d2h[vals[1]] + d2h[vals[2]] + d2h[vals[3]] +
			'-' + d2h[vals[4]] + d2h[vals[5]] +
			'-' + d2h[vals[6]] + d2h[vals[7]] +
			'-' + d2h[vals[8]] + d2h[vals[9]] +
			'-' + d2h[vals[10]] + d2h[vals[11]] + d2h[vals[12]] + d2h[vals[13]] + d2h[vals[14]] + d2h[vals[15]];
	}

	private fillDummyData() {
		this.book.server.update(s => { s.name = "Server der Verplanten"; return s; });
	}

	public sendMessage(data: OutMsg) {
		this.socket?.send(JSON.stringify(data));
	}

	public sendRawMessage(data: string) {
		this.socket?.send(data);
	}

	public disconnect(reason?: Reason, message?: string) {
		this.sendMessage({ Disconnect: { reason, message } });
	}

	public switchChannel(channel: Channel) {
		this.sendMessage({ SwitchChannel: channel.id });
	}

	private messageHandler(evt: MessageEvent) {
		const msg = JSON.parse(evt.data) as InMsg;
		if ("Connected" in msg) {
			this.state.set(ConnectionState.Connected);
			this.server = msg.Connected.server;
			this.ownClient = msg.Connected.own_client;
		} else if ("Events" in msg) {
			for (const tsevt of msg.Events) {
				try {
					console.log(tsevt);
					if (tsevt === "ChannelListFinished") {
					} else if ("Message" in tsevt) {
						// TODO Update chat
					} else {
						this.book.messageHandler(tsevt);
					}
				} catch (err) {
					console.error("Failed to handle event", tsevt, err);
				}
			}
		} else if ("TalkersChanged" in msg) {
			// TODO
		} else if ("Error" in msg) {
			console.warn("Con Error:", msg.Error);
			if (get(this.state) == ConnectionState.Connecting) {
				this.socket?.close();
				this.error.set(msg.Error);
			}
		} else {
			console.error("Unknown message", msg);
		}
	}

	private takenumber(a: number) {}
}

export enum ConnectionState {
	Disconnected,
	Connecting,
	Connected,
}

interface IConnectOptions {
	address: string;
	name: string;
	// ...
}
