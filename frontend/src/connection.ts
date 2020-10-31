import { OutMsg, InMsg } from "./backend/ws";
import { get, writable, Writable, Readable } from "svelte/store";
import { Book, Channel, ChatData } from "./book";
import { getStringFromConnect, oneshot } from "./util";
import { handleMessage } from "./notification";
import { backend, IBackendConnection } from "./backend/backend";
import { app } from "./app";
import { ConnectData } from "./connect/connect";
import { Reason } from "./book_events";
import moment from "moment";
import { ChannelId, ClientId } from "./ts";
import { FileTreeCache } from "./fileTreeCache";
import { DescriptionMode } from "./transientSettings";

export class Connection {
	private readonly _state = writable(new ConnectionState());
	public get state(): Readable<ConnectionState> { return this._state; };

	public readonly book: Book = new Book();
	public readonly fileTreeCache: Writable<FileTreeCache> = writable(new FileTreeCache());
	public backend: IBackendConnection;

	public loudness: Writable<number> = writable(0);
	public connectOptions: ConnectData;

	constructor(connectOptions: ConnectData) {
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
			this.backend.send(this.connectOptions.toConnectMsg());
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

	public close() {
		this.backend.close();
		this._state.update(s => s.setDisconnected());
	}

	private onClose() {
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

	public moveClient(clientId: ClientId, channelId: ChannelId) {
		this.sendMessage({
			Change: {
				ClientMove: {
					id: clientId,
					channel: channelId,
				}
			}
		});
	}

	public moveChannel(moveChannelId: ChannelId, targetParentId: ChannelId, targetOrderId: ChannelId) {
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

	private async updateAllUnreadCounts() {
		// Server
		const serverData = await backend.graphql(`query GetUnreadCounts($server: [Int!]!) {
			chat(typ: SERVER, server: $server) {
				lastRead
				timezone
				unreadCount
			}
		}`, {
			server: this.book.server.publicKey,
		});
		if (serverData.data.chat !== null)
			this.book.server.updateChat(ChatData.fromGraphql(serverData.data.chat));


		// Channels
		const channelData = await backend.graphql(`query GetUnreadCounts($server: [Int!]!) {
			server(server: $server) {
				channels(includeDeleted: false) {
					id
					chat {
						lastRead
						timezone
						unreadCount
					}
				}
			}
		}`, {
			server: this.book.server.publicKey,
		});
		for (const channel of channelData.data.server.channels) {
			if (channel.chat !== null)
				this.book.channels.get(channel.id)!.updateChat(ChatData.fromGraphql(channel.chat));
		}

		// Clients
		for (const client of this.book.clients.values()) {
			const clientData = await backend.graphql(`query GetUnreadCount($server: [Int!]!, $client: ID!) {
				chat(typ: CLIENT, server: $server, id: $client) {
					lastRead
					timezone
					unreadCount
				}
			}`, {
				server: this.book.server.publicKey,
				client: client.uidStr,
			});
			if (clientData.data.chat !== null)
				client.updateChat(ChatData.fromGraphql(clientData.data.chat));
		}
	}

	private async updateClientUnreadCount(clientId: ClientId) {
		const client = this.book.getClient(clientId)!;
		const clientData = await backend.graphql(`query GetUnreadCount($server: [Int!]!, $client: ID!) {
			chat(typ: CLIENT, server: $server, id: $client) {
				lastRead
				timezone
				unreadCount
			}
		}`, {
			server: this.book.server.publicKey,
			client: client.uidStr,
		});
		if (clientData.data.chat !== null)
			client.updateChat(ChatData.fromGraphql(clientData.data.chat));
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
			this.book.server.update({ uid: msg.Connected.server });
			this.book.ownClientId = msg.Connected.ownClient;
		} else if ("DisconnectedTemporarily" in msg) {
			this._state.update(s => s.setConnecting());
			this.book.reset();
		} else if ("Events" in msg) {
			for (const tsevt of msg.Events) {
				try {
					if (get(app.transientSettings.ui._developMode))
						console.log(tsevt);

					if ("Message" in tsevt) {
						const fromOwnClient = tsevt.Message.invoker.id.toString() === this.book.ownClientId;
						let chat = undefined;
						if (tsevt.Message.target === "Server") {
							chat = this.book.server.chat;
						} else if (tsevt.Message.target === "Channel") {
							const ownClient = get(this.book.ownClient);
							if (ownClient !== undefined) {
								const channel = this.book.getChannel(ownClient.channel)!;
								chat = channel.chat;
							}
						} else if ("Client" in tsevt.Message.target || "Poke" in tsevt.Message.target) {
							const targetClientId = "Client" in tsevt.Message.target ? tsevt.Message.target.Client : tsevt.Message.target.Poke;
							const chatClientId = fromOwnClient ? targetClientId : tsevt.Message.invoker.id;
							const client = this.book.getClient(chatClientId.toString());
							if (client !== undefined)
								chat = client.chat;
						}

						if (chat !== undefined)
							chat.update(c => {
								// Only increment unread count for messages from others
								if (fromOwnClient)
									return new ChatData(moment(), c.unreadCount);
								else
									return c.incrementUnread();
							});
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
							} else if (tsevt.PropertyAdded.prop !== undefined &&
								"Client" in tsevt.PropertyAdded.id) {
								this.updateClientUnreadCount(tsevt.PropertyAdded.id.Client);
							}
						} else if ("PropertyChanged" in tsevt) {
							const prop = tsevt.PropertyChanged.prop!;
							if ("Client" in prop && "Client" in tsevt.PropertyChanged.id
								&& tsevt.PropertyChanged.id.Client === this.book.ownClientId
								&& "channel" in prop.Client) {
								// Update selected node
								const curTarget = get(app.selectedNode);
								if (curTarget === undefined || curTarget.node.qlType === "CHANNEL")
									app.select(this, this.book.getChannel(prop.Client.channel!)!);
							}
						}
					}
				} catch (err) {
					console.error("Failed to handle event", tsevt, err);
				}
			}
		} else if ("Message" in msg) {
			const message = msg.Message;
			if (get(app.transientSettings.ui._developMode))
				console.log(message);

			if ("ChannelDescriptionChanged" in message) {
				const curTarget = get(app.selectedNode);
				if (curTarget !== undefined && curTarget.connection === this && curTarget.node.qlType === "CHANNEL"
					&& get(app.transientSettings.ui._descriptionMode) === DescriptionMode.Info) {
					for (const c of message.ChannelDescriptionChanged) {
						if (c.channelId === curTarget.node.qlId) {
							// Update channel description
							this.sendMessage({ Change: {
								ChannelDescriptionRequest: {
									id: c.channelId,
								},
							}});
							break;
						}
					}
				}
			} else if ("ChannelListFinished" in message) {
				this._state.update(s => s.setChannelListFinished());
				location.hash = getStringFromConnect(this.connectOptions!);
				this.updateAllUnreadCounts();
			} else if ("FileList" in message) {
				this.fileTreeCache.update(ftc => ftc.applyFileList(message));
			} else if ("ServerEdited" in message) {
				// TODO We do not get this message because it is a book message...
				this.sendMessage({ Change: {
					ServerVariablesRequest: {},
				}});
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
