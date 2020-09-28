import { Chat } from "./chat/chat";
import { OutMsg, OMsgConnect, InMsg, Reason } from "./backend/ws";
import { derived, get, writable, Readable, Writable } from "svelte/store";
import { Book, Channel, Client } from "./book";
import { plugins, loadPlugins } from "./plugins";
import { getStringFromConnect } from "./util";
import { handleMessage } from "./notification";
import { transientSettings } from "./transientSettings";
import { backend, IBackendConnction } from "./backend/backend";

export class Connection {
	public readonly state = writable(ConnectionState.Disconnected);
	public readonly error: Writable<string | undefined> = writable(undefined);

	public readonly book: Book = new Book();
	public readonly chat: Chat = new Chat(this);
	public server?: string;
	public ownClientId?: number;
	public ownClient: Readable<Client | undefined> = derived(this.book.clients,
		cls => this.ownClientId !== undefined ? cls.get(this.ownClientId) : undefined);
	public backend: IBackendConnction;
	private connectOptions: OMsgConnect | undefined;

	private muted: boolean = false;
	public loudness: Writable<number> = writable(0);

	constructor() {
		this.backend = backend.createNewConnection();
		this.book.server.subscribe(s => {
			if (s === undefined || s.name === undefined)
				backend.setTitle("Qint")
			else
				backend.setTitle(s.name + " – Qint")
		});
		loadPlugins();
		transientSettings.read_from_proxy();
	}

	public reset() {
		this.state.set(ConnectionState.Disconnected);
		this.book.reset();
		this.chat.reset();
		this.server = undefined;
		this.ownClientId = undefined;
		this.backend.close();
		this.muted = false;
		backend.setTitle("Qint");
		location.hash = "";
	}

	public getState(): ConnectionState {
		return get(this.state);
	}

	public isMuted(): boolean {
		return this.muted;
	}

	public connect(opt: OMsgConnect) {
		this.error.set(undefined);
		this.connectOptions = opt;
		this.state.set(ConnectionState.Connecting);
		this.backend.connect(
			(msg) => { this.messageHandler(msg) },
			(err) => { this.error.set(`Connection failed, is Qint running? (${err})`); },
			() => {
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
			},
		).then(() => {
			this.backend.send(opt)
		});
	}

	public sendMessage(data: OutMsg) {
		this.backend.send(data);
	}

	public disconnect(reason?: Reason, message?: string) {
		this.sendMessage({ Disconnect: { reason, message } });
	}

	public switchChannel(channel: Channel) {
		this.moveClient(this.ownClientId!, channel.id);
	}

	public moveClient(clientId: number, channelId: number) {
		this.sendMessage({
			Change: {
				ClientMove: {
					id: clientId,
					channel: channelId,
				}
			}
		});
	}

	public moveChannel(moveChannelId: number, targetParentId: number, targetOrderId: number) {
		this.sendMessage({
			Change: {
				ChannelMove: {
					id: moveChannelId,
					parent: targetParentId,
					order: targetOrderId,
				}
			}
		});
	}

	private messageHandler(msg: InMsg) {
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
						location.hash = getStringFromConnect(this.connectOptions!);
						// TODO Get unread counts for channels and clients
					} else if ("Message" in tsevt) {
						this.chat.unreadCount.update(c => c + 1);
					} else {
						if ("PropertyRemoved" in tsevt) {
							if ("Client" in tsevt.PropertyRemoved.id) {
								if (tsevt.PropertyRemoved.id.Client === this.ownClientId) {
									this.reset();
									return;
								}
							}
						}

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
				this.backend.close();
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
