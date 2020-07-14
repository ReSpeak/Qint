import { Chat } from "./chat/chat";
import { OutMsg, InMsg, Reason } from "./structs/ws";
import { derived, get, writable, Readable, Writable } from "svelte/store";
import { Book, Channel, Client } from "./tree/book";
import { plugins, loadPlugins } from "./plugins";
import { BASE_ADDRESS } from "./util";
import { handleMessage } from "./notification";

export class Connection {
	public readonly state = writable(ConnectionState.Disconnected);
	public readonly error: Writable<string | undefined> = writable(undefined);

	public readonly book: Book = new Book();
	public readonly chat: Chat = new Chat(this);
	public server?: string;
	public ownClientId?: number;
	public ownClient: Readable<Client | undefined> = derived(this.book.clients,
		cls => this.ownClientId !== undefined ? cls.get(this.ownClientId) : undefined);
	private socket?: WebSocket;
	public guid?: string;
	private muted: boolean = false;
	public loudness: Writable<number> = writable(0);

	constructor() {
		loadPlugins();
		(window as any).mov = (a: any, b: any, c: any) => this.moveChannel(a, b, c);
	}

	public reset() {
		this.state.set(ConnectionState.Disconnected);
		this.book.reset();
		this.chat.reset();
		this.server = undefined;
		this.ownClientId = undefined;
		this.guid = undefined;
		if (this.socket)
			this.socket.close();
		this.socket = undefined;
		this.muted = false;
		document.title = "Qint";
	}

	public getState(): ConnectionState {
		return get(this.state);
	}

	public isMuted(): boolean {
		return this.muted;
	}

	public connect(opt: IConnectOptions) {
		this.error.set(undefined);
		this.guid = Connection.createUuidV4();
		let path = BASE_ADDRESS;
		if (!path.startsWith("http"))
			path = window.location.origin;
		if (!path.startsWith("http"))
			throw Error("Failed to get websocket path");
		// Replace http by ws, so https gets wss
		path = path.slice(4);

		this.socket = new WebSocket(`ws${path}/con/${this.guid}/ws?format=Json`);
		this.socket.onopen = () => {
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
			this.sendMessage({
				Connect: {
					address: opt.address,
					name: opt.name,
					log_commands: false,
					log_packets: false,
					log_udp_packets: false,
					version,
				}
			});
		};
		this.socket.onerror = (error) => {
			this.error.set("Connection failed, is Qint running?");
		};
		this.socket.onclose = () => {
			// Plugins
			for (const plugin of plugins) {
				try {
					if ("handleEvent" in plugin) {
						plugin.handleEvent(this, { Disconnected: null });
					}
				} catch (e) {
					console.error("Failed to handle event in plugin:", e);
				}
			}
			this.reset();
		};
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

	public sendMessage(data: OutMsg) {
		if (this.socket)
			this.socket.send(JSON.stringify(data));
	}

	public sendRawMessage(data: string) {
		if (this.socket)
			this.socket.send(data);
	}

	public disconnect(reason?: Reason, message?: string) {
		this.sendMessage({ Disconnect: { reason, message } });
	}

	public switchChannel(channel: Channel) {
		this.moveClient(this.ownClientId!, channel.id);
	}

	public moveClient(clientId: number, channelId: number) {
		this.sendMessage({
			Events: [{
				PropertyChanged: {
					id: {
						Client: clientId,
					},
					prop: {
						Client: {
							channel: channelId,
						},
					},
					invoker: null,
					extra: { reason: null },
				}
			}]
		});
	}

	public moveChannel(moveChannelId: number, targetChannelId?: number, targetOrder?: number) {
		this.sendMessage({
			Events: [{
				PropertyChanged: {
					id: {
						Channel: moveChannelId,
					},
					prop: {
						Channel: {
							parent: targetChannelId,
							order: targetOrder,
						},
					},
					invoker: null,
					extra: { reason: null },
				}
			}]
		});
	}

	private messageHandler(evt: MessageEvent) {
		const msg = JSON.parse(evt.data) as InMsg;
		// Plugins
		for (const plugin of plugins) {
			try {
				if ("handleEvent" in plugin) {
					plugin.handleEvent(this, msg);
				}
			} catch (e) {
				console.error("Failed to handle event in plugin:", e);
			}
		}

		handleMessage(this, msg, plugins);
		if ("Connected" in msg) {
			this.state.set(ConnectionState.Connected);
			this.server = msg.Connected.server;
			this.ownClientId = msg.Connected.own_client;
		} else if ("DisconnectedTemporarily" in msg) {
			this.state.set(ConnectionState.Connecting);
			this.book.reset();
			this.chat.reset();
			this.server = undefined;
			this.ownClientId = undefined;
		} else if ("Events" in msg) {
			for (const tsevt of msg.Events) {
				try {
					console.log(tsevt);
					if (tsevt === "ChannelListFinished") {
						this.state.set(ConnectionState.ChannelListFinished);
					} else if ("Message" in tsevt) {
						this.chat.unreadCount.update(c => c + 1);
					} else {
						this.book.messageHandler(tsevt);
					}
				} catch (err) {
					console.error("Failed to handle event", tsevt, err);
				}
			}
		} else if ("TalkersChanged" in msg) {
			this.book.talkersHandler(msg.TalkersChanged);
		} else if ("Error" in msg) {
			console.warn("Con Error:", msg.Error);
			if (get(this.state) === ConnectionState.Connecting) {
				if (this.socket)
					this.socket.close();
				this.error.set(msg.Error);
			}
		} else if ("Loudness" in msg) {
			this.loudness.update(_ => msg.Loudness);
		} else {
			console.error("Unknown message", msg);
		}
	}
}

export enum ConnectionState {
	Disconnected,
	Connecting,
	Connected,
	ChannelListFinished,
}

interface IConnectOptions {
	address: string;
	name: string;
	// ...
}
