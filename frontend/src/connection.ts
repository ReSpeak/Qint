import { Chat } from "./chat/chat";
import { OutMsg, OMsgConnect, InMsg, Reason } from "./backend/ws";
import { get, writable, Writable } from "svelte/store";
import { Book, Channel } from "./book";
import { getStringFromConnect } from "./util";
import { handleMessage } from "./notification";
import { backend, IBackendConnection } from "./backend/backend";
import { app } from "./app";

export class Connection {
	public readonly state = writable(ConnectionState.Disconnected);
	public readonly error: Writable<string | undefined> = writable(undefined);

	public readonly book: Book = new Book();
	public chat: Chat | undefined;
	public server?: string;
	public backend: IBackendConnection;

	private muted: boolean = false;
	public loudness: Writable<number> = writable(0);
	private connectOptions: OMsgConnect;

	constructor(connectOptions: OMsgConnect) {
		this.connectOptions = connectOptions;
		this.backend = backend.createNewConnection();
		this.error.set(undefined);
		this.state.set(ConnectionState.Connecting);
		this.backend.connect(
			(msg) => { this.messageHandler(msg) },
			(err) => { this.error.set(`Connection failed, is Qint running? (${err})`); },
			() => this.onClose(),
		).then(() => {
			this.backend.send(this.connectOptions);
		});
	}

	public reset() {
		this.state.set(ConnectionState.Disconnected);
		this.book.reset();
		this.server = undefined;
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

	public onClose() {
		// Plugins
		for (const plugin of app.plugins) {
			try {
				plugin.handleEvent?.(this, { Disconnected: null });
			} catch (e) {
				console.error("Failed to handle event in plugin:", e);
			}
		}

		this.reset();
	}

	public sendMessage(data: OutMsg) {
		this.backend.send(data);
	}

	public disconnect(reason?: Reason, message?: string) {
		this.sendMessage({ Disconnect: { reason, message } });
	}

	public switchChannel(channel: Channel) {
		this.moveClient(this.book.ownClientId!, channel.id);
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
		for (const plugin of app.plugins) {
			try {
				plugin.handleEvent?.(this, msg);
			} catch (e) {
				console.error("Failed to handle event in plugin:", e);
			}
		}

		handleMessage(this, msg, app.plugins);
		if ("Connected" in msg) {
			this.server = msg.Connected.server;
			this.book.ownClientId = msg.Connected.own_client;
		} else if ("DisconnectedTemporarily" in msg) {
			this.state.set(ConnectionState.Connecting);
			this.book.reset();
			this.server = undefined;
		} else if ("Events" in msg) {
			for (const tsevt of msg.Events) {
				try {
					console.log(tsevt);
					if (tsevt === "ChannelListFinished") {
						this.state.set(ConnectionState.ChannelListFinished);
						location.hash = getStringFromConnect(this.connectOptions!);
						// TODO Get unread counts for channels and clients
					} else if ("Message" in tsevt) {
						this.chat?.unreadCount.update(c => c + 1);
					} else {
						if ("PropertyRemoved" in tsevt) {
							if ("Client" in tsevt.PropertyRemoved.id) {
								if (tsevt.PropertyRemoved.id.Client === this.book.ownClientId) {
									this.reset();
									return;
								}
							}
						}
						this.book.messageHandler(tsevt);
						if ("PropertyAdded" in tsevt) {
							if (tsevt.PropertyAdded.prop !== undefined &&
								"Server" in tsevt.PropertyAdded.prop) {
								this.state.set(ConnectionState.Connected);
							}
						}
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
