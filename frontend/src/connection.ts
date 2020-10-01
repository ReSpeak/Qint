import { OutMsg, OMsgConnect, InMsg, Reason } from "./backend/ws";
import { get, writable, Writable, Readable } from "svelte/store";
import { Book, Channel } from "./book";
import { getStringFromConnect, oneshot } from "./util";
import { handleMessage } from "./notification";
import { backend, IBackendConnection } from "./backend/backend";
import { app } from "./app";

export class Connection {
	private readonly _state = writable(new ConnectionState());
	public get state(): Readable<ConnectionState> { return this._state; };

	public readonly book: Book = new Book();
	public server?: string;
	public backend: IBackendConnection;

	public loudness: Writable<number> = writable(0);
	public connectOptions: OMsgConnect;

	constructor(connectOptions: OMsgConnect) {
		this.connectOptions = connectOptions;
		this.backend = backend.createNewConnection();
		this._state.update(s => s.setConnecting());
		this.backend.connect(
			(msg) => { this.messageHandler(msg) },
			(err) => {
				this._state.update(s => s.setError(`Connection failed, is Qint running? (${err})`));
			},
			() => this.onClose(),
		).then(() => {
			this.backend.send(this.connectOptions);
			oneshot(this.state, s => s.channelListFinished, () => {
				const ownClient = get(this.book.ownClient);
				if (ownClient === undefined) return;
				const ownChannel = this.book.getChannel(ownClient.channel);
				if (ownChannel === undefined) return;
				app.select(this, ownChannel);
			})
		});
	}

	public getState(): Readonly<ConnectionState> {
		return get(this.state);
	}

	// TODO recheck for sanity close -> onClose, or onClose -> close
	public close() {
		this.backend.close();
		this._state.update(s => s.setDisconnected());
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
		location.hash = "";
		// Reset chat if the selected node is from this connection.
		app.selectedNode.update(n => n?.connection === this ? undefined : n);
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
			this._state.update(s => s.setConnecting());
			this.book.reset();
			this.server = undefined;
		} else if ("Events" in msg) {
			for (const tsevt of msg.Events) {
				try {
					console.log(tsevt);
					if (tsevt === "ChannelListFinished") {
						this._state.update(s => s.setChannelListFinished());
						location.hash = getStringFromConnect(this.connectOptions!);
						// TODO Get unread counts for channels and clients
					} else if ("Message" in tsevt) {
						app.chat.unreadCount.update(c => c + 1);
					} else {
						if ("PropertyRemoved" in tsevt) {
							if ("Client" in tsevt.PropertyRemoved.id) {
								if (tsevt.PropertyRemoved.id.Client === this.book.ownClientId) {
									this.close();
									return;
								}
							}
						}
						this.book.messageHandler(tsevt);
						if ("PropertyAdded" in tsevt) {
							if (tsevt.PropertyAdded.prop !== undefined &&
								"Server" in tsevt.PropertyAdded.prop) {
								this._state.update(s => s.setConnected());
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
			if (this.getState().connecting) {
				this.backend.close(); // TODO call general close
				this._state.update(s => s.setError(msg.Error));
			}
		} else if ("Loudness" in msg) {
			this.loudness.set(msg.Loudness);
		} else {
			console.error("Unknown message", msg);
		}
	}
}

export enum ConnectionStateEnum {
	Uninitialized,
	Connecting,
	Connected,
	ChannelListFinished,
	Disconnected,
	Errored,
}

export class ConnectionState {
	public rawState: ConnectionStateEnum = ConnectionStateEnum.Uninitialized;
	public error: string | undefined;
	public get channelListFinished() { return this.rawState === ConnectionStateEnum.ChannelListFinished; }
	public get connecting() { return this.rawState === ConnectionStateEnum.Connecting; }
	public get connected() {
		return this.rawState === ConnectionStateEnum.Connected ||
			this.rawState === ConnectionStateEnum.ChannelListFinished;
	}
	public get errored() { return this.rawState === ConnectionStateEnum.Errored; }
	public get closed() { return this.rawState === ConnectionStateEnum.Disconnected; }

	public setConnecting(): this {
		if (this.rawState !== ConnectionStateEnum.Uninitialized
			&& this.rawState !== ConnectionStateEnum.Connected
			&& this.rawState !== ConnectionStateEnum.ChannelListFinished)
			this.throwTransition(ConnectionStateEnum.Connecting);
		this.rawState = ConnectionStateEnum.Connecting;
		return this;
	}

	public setConnected(): this {
		if (this.rawState !== ConnectionStateEnum.Connecting)
			this.throwTransition(ConnectionStateEnum.Connected);
		this.rawState = ConnectionStateEnum.Connected;
		return this;
	}

	public setChannelListFinished(): this {
		if (this.rawState !== ConnectionStateEnum.Connected)
			this.throwTransition(ConnectionStateEnum.ChannelListFinished);
		this.rawState = ConnectionStateEnum.ChannelListFinished;
		return this;
	}

	public setDisconnected(): this {
		this.rawState = ConnectionStateEnum.Disconnected;
		return this;
	}

	public setError(msg: string): this {
		this.rawState = ConnectionStateEnum.Errored;
		this.error = msg;
		return this;
	}

	private throwTransition(newState: ConnectionStateEnum): never {
		throw Error(`Cannot transition this connection from '${
			ConnectionStateEnum[this.rawState]}' to ${
			ConnectionStateEnum[newState]}`);
	}
}
