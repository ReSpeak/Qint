import { ResultDetails, OutMsg, InMsg } from "./backend/ws";
import { get, writable, Writable, Readable } from "svelte/store";
import { Book, Channel, ChatData, Client } from "./book";
import { oneshot, fnBroadcast, LOUDNESS_MIN, Lazy } from "./util";
import { handleMessage } from "./notifications";
import { backend, IBackendConnection, IFileRequest } from "./backend/backend";
import { app } from "./app";
import { ConnectData, MuteState } from "./connect/uiConnect";
import {
	OChange,
	Reason,
	IMsgPluginCommandPart,
	IMsgServerLogPart,
	IMsgPermListPart,
	IMsgChannelPermListPart,
} from "./book_events";
import moment from "moment";
import { ChannelId, ClientId, Permission, PermissionDescription } from "./ts";
import { FileTreeCache } from "./fileTreeCache";
import debug from "debug";
import VoiceGraph from "./ui/specialized/VoiceGraph.svelte";
import { ChangePromise, ReturnCodeTracker } from "./backend/returnCodeTracker";
const log_raw_in = debug("RAW:IN");
const log_raw_out = debug("RAW:OUT");
const log = debug("CON"),
	error = debug("error:CON");
const log_evt = log.extend("EVT"),
	log_msg = log.extend("MSG");

export interface IConnection {
	isOnline(): this is Connection;
	fileProvider(req: IFileRequest): Promise<string | undefined>;
}

export class Connection implements IConnection {
	private readonly _state = writable(new ConnectionState());
	private returnCodes = new ReturnCodeTracker();
	private permListResult: IMsgPermListPart[] | undefined;
	public get state(): Readable<ConnectionState> {
		return this._state;
	}
	private _isWhispering = false;
	public get isWhispering(): boolean {
		return this._isWhispering;
	}

	public readonly book: Book = new Book();
	public readonly channelPermCache: Writable<IMsgChannelPermListPart[]> = writable([]);
	public readonly fileTreeCache: Writable<FileTreeCache> = writable(new FileTreeCache());
	public backend: IBackendConnection;

	public readonly loudness: Writable<number> = writable(0);
	public readonly connectOptions: Writable<ConnectData>;
	public readonly permList: Lazy<Promise<Map<Permission, PermissionDescription>>>;
	public pluginCmd = fnBroadcast<[IMsgPluginCommandPart]>();
	public serverLogCmd = fnBroadcast<[IMsgServerLogPart[]]>();

	/** Listeners for loudness from the ui. */
	public readonly loudnesses: Map<ClientId, VoiceGraph> = new Map();

	constructor(connectOptions: ConnectData) {
		this.connectOptions = writable(connectOptions);
		this.backend = backend.createNewConnection(this.returnCodes);
		this._state.update((s) => s.setConnecting());
		this.backend
			.connect(
				(msg) => {
					this.messageHandler(msg);
				},
				(err) => {
					this._state.update((s) =>
						s.setError(`Connection failed, is Qint running? (${err})`)
					);
				},
				() => this.onClose()
			)
			.then(() => {
				const connectMsg = connectOptions.toConnectMsg();
				const [returnCode, promise] = this.returnCodes.getNew();
				connectMsg.Connect.returnCode = returnCode;
				this.backend.send(connectMsg);
				oneshot(
					this.state,
					(s) => s.channelListFinished,
					() => {
						const ownClient = get(this.book.ownClient);
						if (ownClient === undefined) return;
						const ownChannel = this.book.getChannel(ownClient.channel);
						if (ownChannel === undefined) return;
						app.select(this, ownChannel);
					}
				);

				promise.then((res) => {
					if (res !== undefined) {
						this.backend.close();
						this._state.update((s) => s.setError(res));
					}
				});
			});
		this.permList = new Lazy(async () => {
			await this.sendChange({ ServerPermListRequest: {} });
			if (this.permListResult === undefined) {
				console.error("Failed to get permission list");
				return new Map();
			}
			const res: Map<Permission, PermissionDescription> = new Map();
			let i = 1;
			for (const perm of this.permListResult) {
				if (perm.permissionName !== undefined) {
					res.set(i, {
						name: perm.permissionName,
						description: perm.permissionDescription,
					});
					i++;
				}
			}
			return res;
		});
	}

	isOnline(): boolean {
		return true;
	}

	public getState(): Readonly<ConnectionState> {
		return get(this.state);
	}

	public close(): void {
		this.backend.close();
		this._state.update((s) => s.setDisconnected());

		app.clientsByUid.update((clients) => {
			for (const c of this.book.clients.values()) {
				if (c.uid !== null) {
					const cs = clients.get(c.uidStr);
					if (cs !== undefined) {
						for (let i = 0; i < cs.length; i++) {
							if (cs[i][0] === this) {
								cs.splice(i, 1);
							}
						}
						if (cs.length === 0) clients.delete(c.uidStr);
					}
				}
			}
			return clients;
		});
		if (this.book.server !== undefined) {
			app.serversByUid.update((servers) => {
				servers.delete(this.book.server.uidStr);
				return servers;
			});
		}
	}

	private onClose(): void {
		if (!this.getState().errored) this._state.update((s) => s.setDisconnected());
		// Plugins
		for (const plugin of app.plugins) {
			try {
				plugin.handleEvent?.(this, { Disconnected: null });
			} catch (e) {
				error("Failed to handle event in plugin: %o", e);
			}
		}
		// Reset chat if the selected node is from this connection.
		app.updateSelections((sels) => sels.filter((sel) => sel.connection !== this));
		this.returnCodes.rejectAll();
	}

	public sendMessage(data: OutMsg): void {
		log_raw_out("%o", data);
		this.backend.send(data);
	}

	public sendChange(change: OChange): ChangePromise {
		const [returnCode, promise] = this.returnCodes.getNew();
		this.sendMessage({
			Change: {
				change,
				returnCode: returnCode,
			},
		});
		return promise;
	}

	public disconnect(reason?: Reason, message?: string): void {
		this.sendMessage({ Disconnect: { reason, message } });
	}

	public switchChannel(channel: Channel, password?: string): ChangePromise {
		return this.moveClient(this.book.ownClientId!, channel.id, password);
	}

	public moveClient(clientId: ClientId, channelId: ChannelId, password?: string): ChangePromise {
		return this.sendChange({
			ClientMove: {
				id: clientId,
				channel: channelId,
				password,
			},
		});
	}

	public moveChannel(
		moveChannelId: ChannelId,
		targetParentId: ChannelId,
		targetOrderId: ChannelId
	): ChangePromise {
		return this.sendChange({
			ChannelMove: {
				id: moveChannelId,
				parent: targetParentId,
				order: targetOrderId,
			},
		});
	}

	public pokeClient(clientId: ClientId, message: string): void {
		// TODO Use return code
		this.sendMessage({
			SendMessage: {
				target: {
					Poke: clientId,
				},
				message,
			},
		});
		// Update chat
		const client = this.book.getClient(clientId);
		if (client !== undefined) {
			client.chat.set(new ChatData(moment(), 0));
		}
	}

	public startWhispering(clients: ClientId[], channels: ChannelId[]): void {
		this.sendMessage({
			SetWhispering: {
				channels,
				clients,
			},
		});
		this._isWhispering = true;
	}

	public stopWhispering(): void {
		if (this._isWhispering) {
			this.sendMessage({ SetWhispering: null });
			this._isWhispering = false;
		}
	}

	private async updateAllUnreadCounts(): Promise<void> {
		// Server
		const serverData = await backend.graphql(
			`query GetUnreadCounts($server: [Int!]!) {
			chat(typ: SERVER, server: $server) {
				lastRead
				timezone
				unreadCount
			}
		}`,
			{
				server: this.book.server.publicKey,
			}
		);
		if (serverData.data.chat !== null)
			this.book.server.updateChat(ChatData.fromGraphql(serverData.data.chat));

		// Channels
		const channelData = await backend.graphql(
			`query GetUnreadCounts($server: [Int!]!) {
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
		}`,
			{
				server: this.book.server.publicKey,
			}
		);
		for (const channel of channelData.data.server.channels) {
			if (channel.chat !== null)
				this.book.channels.get(channel.id)!.updateChat(ChatData.fromGraphql(channel.chat));
		}

		// Clients
		for (const client of this.book.clients.values()) {
			const clientData = await backend.graphql(
				`query GetUnreadCount($server: [Int!]!, $client: ID!) {
				chat(typ: CLIENT, server: $server, id: $client) {
					lastRead
					timezone
					unreadCount
				}
			}`,
				{
					server: this.book.server.publicKey,
					client: client.uidStr,
				}
			);
			if (clientData.data.chat !== null)
				client.updateChat(ChatData.fromGraphql(clientData.data.chat));
		}
	}

	private async updateClientUnreadCount(clientId: ClientId): Promise<void> {
		const client = this.book.getClient(clientId)!;
		const clientData = await backend.graphql(
			`query GetUnreadCount($server: [Int!]!, $client: ID!) {
			chat(typ: CLIENT, server: $server, id: $client) {
				lastRead
				timezone
				unreadCount
			}
		}`,
			{
				server: this.book.server.publicKey,
				client: client.uidStr,
			}
		);
		if (clientData.data.chat !== null)
			client.updateChat(ChatData.fromGraphql(clientData.data.chat));
	}

	private renderRequested = false;

	private applyLoudnesses(loudnesses: Record<ClientId, number>): void {
		const now = performance.now();

		// eslint-disable-next-line prefer-const
		for (let [client, loudness] of Object.entries(loudnesses)) {
			const l = this.loudnesses.get(client);
			if (l !== undefined) {
				loudness = Math.max(loudness, LOUDNESS_MIN + 2);
				l.addValue(loudness, now);
			}
		}

		this.requestRenderLoudnessGraphs();
	}

	private requestRenderLoudnessGraphs(): void {
		if (this.renderRequested) return;
		this.renderRequested = true;
		requestAnimationFrame((ts) => this.renderLoudnessGraphs(ts));
	}

	private renderLoudnessGraphs(timestamp: number): void {
		this.renderRequested = false;
		let hasRequest = false;
		for (const hist of this.loudnesses.values()) {
			const requestNextFrame = hist.redraw(timestamp);
			hasRequest ||= requestNextFrame;
		}
		if (hasRequest) {
			this.requestRenderLoudnessGraphs();
		}
	}

	private messageHandler(msg: InMsg): void {
		log_raw_in("%o", msg);

		// Plugins
		for (const plugin of app.plugins) {
			try {
				plugin.handleEvent?.(this, msg);
			} catch (e) {
				error("Failed to handle event in plugin:%o", e);
			}
		}

		handleMessage(this, msg);
		if ("Connected" in msg) {
			this.book.server.update({ uid: msg.Connected.server });
			this.book.ownClientId = msg.Connected.ownClient;
		} else if ("DisconnectedTemporarily" in msg) {
			this._state.update((s) => s.setConnecting());
			this.book.reset();
			this.returnCodes.rejectAll();
		} else if ("Events" in msg) {
			for (const tsevt of msg.Events) {
				try {
					log_evt("%o", tsevt);

					if ("Message" in tsevt) {
						const fromOwnClient =
							tsevt.Message.invoker.id.toString() === this.book.ownClientId;
						let chat = undefined;
						if (tsevt.Message.target === "Server") {
							chat = this.book.server.chat;
						} else if (tsevt.Message.target === "Channel") {
							const ownClient = get(this.book.ownClient);
							if (ownClient !== undefined) {
								const channel = this.book.getChannel(ownClient.channel)!;
								chat = channel.chat;
							}
						} else if (
							"Client" in tsevt.Message.target ||
							"Poke" in tsevt.Message.target
						) {
							const targetClientId =
								"Client" in tsevt.Message.target
									? tsevt.Message.target.Client
									: tsevt.Message.target.Poke;
							const chatClientId = fromOwnClient
								? targetClientId
								: tsevt.Message.invoker.id;
							const client = this.book.getClient(chatClientId.toString());
							if (client !== undefined) chat = client.chat;
							log("pok? %o %o %s %s", client, chat, targetClientId, chatClientId);
						}

						if (chat !== undefined)
							if (fromOwnClient)
								// Only increment unread count for messages from others
								chat.set(new ChatData(moment(), 0));
							else chat.update((c) => c.incrementUnread());
					} else {
						if ("PropertyRemoved" in tsevt) {
							if ("Client" in tsevt.PropertyRemoved.id) {
								const id = tsevt.PropertyRemoved.id.Client;
								if (id === this.book.ownClientId) {
									this.close();
									return;
								}
								// Reset chat if the selected node is from this client.
								app.updateSelections((sels) =>
									sels.filter((sel) => {
										return !(
											sel.connection === this &&
											sel.node instanceof Client &&
											sel.node.id === id
										);
									})
								);
								const client = this.book.getClient(id);
								if (client !== undefined && client.uid !== null) {
									app.clientsByUid.update((clients) => {
										const cs = clients.get(client.uidStr);
										if (cs !== undefined) {
											for (let i = 0; i < cs.length; i++) {
												if (cs[i][1] === client) {
													cs.splice(i, 1);
													break;
												}
											}
											if (cs.length === 0) clients.delete(client.uidStr);
										}
										return clients;
									});
								}
							}
						}

						this.book.messageHandler(tsevt);

						if ("PropertyAdded" in tsevt) {
							if (
								tsevt.PropertyAdded.prop !== undefined &&
								"Server" in tsevt.PropertyAdded.prop
							) {
								this._state.update((s) => s.setConnected());
								app.serversByUid.update((servers) => {
									servers.set(this.book.server.uidStr, this);
									return servers;
								});
							} else if (
								tsevt.PropertyAdded.prop !== undefined &&
								"Client" in tsevt.PropertyAdded.id
							) {
								this.updateClientUnreadCount(tsevt.PropertyAdded.id.Client);
								const client = this.book.getClient(tsevt.PropertyAdded.id.Client);
								if (client !== undefined && client.uid !== null) {
									app.clientsByUid.update((clients) => {
										const cs = clients.get(client.uidStr);
										if (cs !== undefined) cs.push([this, client]);
										else clients.set(client.uidStr, [[this, client]]);
										return clients;
									});
								}
							}
						} else if ("PropertyChanged" in tsevt) {
							const prop = tsevt.PropertyChanged.prop!;
							if (
								"Client" in prop &&
								"Client" in tsevt.PropertyChanged.id &&
								tsevt.PropertyChanged.id.Client === this.book.ownClientId
							) {
								if ("channel" in prop.Client) {
									// Update selected node
									const curTarget = get(app.selectedNode);
									if (
										curTarget.selections.some(
											(sel) => sel.node.qlType === "CHANNEL"
										)
									) {
										app.select(
											this,
											this.book.getChannel(prop.Client.channel!)!
										);
									}
								}

								const muteOrAwayChanged =
									"inputMuted" in prop.Client ||
									"inputHardwareEnabled" in prop.Client ||
									"outputMuted" in prop.Client ||
									"outputHardwareEnabled" in prop.Client ||
									"awayMessage" in prop.Client;

								if (
									muteOrAwayChanged ||
									"name" in prop.Client ||
									"channel" in prop.Client
								) {
									this.connectOptions.update((opts) => {
										if ("inputMuted" in prop.Client)
											opts.inputMuted = prop.Client.inputMuted
												? MuteState.Muted
												: undefined;
										if ("inputHardwareEnabled" in prop.Client)
											opts.inputMuted = !prop.Client.inputHardwareEnabled
												? MuteState.Disabled
												: undefined;
										if ("outputMuted" in prop.Client)
											opts.outputMuted = prop.Client.outputMuted
												? MuteState.Muted
												: undefined;
										if ("outputHardwareEnabled" in prop.Client)
											opts.outputMuted = !prop.Client.outputHardwareEnabled
												? MuteState.Disabled
												: undefined;

										if (prop.Client.name !== undefined)
											opts.name = prop.Client.name;
										if (prop.Client.awayMessage !== undefined) {
											if (prop.Client.awayMessage === null)
												opts.away = undefined;
											else opts.away = prop.Client.awayMessage;
										}
										if ("channel" in prop.Client) {
											opts.channel = undefined;
											opts.channelId = prop.Client.channel;
										}
										return opts;
									});
								}

								if (muteOrAwayChanged) app.updateMuteState();
							}
						}
					}
				} catch (err) {
					error("Failed to handle event", tsevt, err);
				}
			}
		} else if ("Message" in msg) {
			const message = msg.Message;
			log_msg("%o", message);

			if ("ChannelListFinished" in message) {
				this._state.update((s) => s.setChannelListFinished());
				this.updateAllUnreadCounts();
			} else if ("ChannelPermList" in message) {
				this.channelPermCache.set(message.ChannelPermList);
			} else if ("FileList" in message) {
				this.fileTreeCache.update((ftc) => ftc.applyFileList(message));
			} else if ("PluginCommand" in message) {
				message.PluginCommand.forEach((pc) => this.pluginCmd(pc));
			} else if ("ServerLog" in message) {
				this.serverLogCmd(message.ServerLog);
			} else if ("PermList" in message) {
				this.permListResult = message.PermList;
			}
		} else if ("TalkersChanged" in msg) {
			this.book.talkersHandler(msg.TalkersChanged);
			this.requestRenderLoudnessGraphs();
		} else if ("Error" in msg) {
			log("Con Error: %o", msg.Error);
			this.backend.close();
			this._state.update((s) => s.setError(msg.Error));
		} else if ("Loudnesses" in msg) {
			this.applyLoudnesses(msg.Loudnesses);
		} else if ("Result" in msg) {
			this.returnCodes.resolve(msg.Result.returnCode, msg.Result);
		} else {
			error("Unknown message", msg);
		}
	}

	// Filetrasfer

	public fileProvider(req: IFileRequest): Promise<string | undefined> {
		return this.backend.fetch_image(req);
	}
}

export class OfflineConnection implements IConnection {
	isOnline(): boolean {
		return false;
	}

	constructor(public server: string) {}

	public fileProvider(req: IFileRequest): Promise<string | undefined> {
		return backend.fetch_cache_image({ server: this.server, ...req });
	}
}

export class NullConnection implements IConnection {
	public static readonly Instance: IConnection = new NullConnection();
	isOnline(): boolean {
		return false;
	}

	private constructor() {}

	public fileProvider(_req: IFileRequest): Promise<undefined> {
		return Promise.resolve(undefined);
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
	public error: string | ResultDetails | undefined;
	public get channelListFinished(): boolean {
		return this.rawState === ConnectionStateEnum.ChannelListFinished;
	}
	public get connecting(): boolean {
		return this.rawState === ConnectionStateEnum.Connecting;
	}
	public get connected(): boolean {
		return (
			this.rawState === ConnectionStateEnum.Connected ||
			this.rawState === ConnectionStateEnum.ChannelListFinished
		);
	}
	public get errored(): boolean {
		return this.rawState === ConnectionStateEnum.Errored;
	}
	public get closed(): boolean {
		return this.rawState === ConnectionStateEnum.Disconnected;
	}

	public setConnecting(): this {
		if (
			this.rawState !== ConnectionStateEnum.Uninitialized &&
			this.rawState !== ConnectionStateEnum.Connected &&
			this.rawState !== ConnectionStateEnum.ChannelListFinished
		)
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

	public setError(msg: string | ResultDetails): this {
		this.rawState = ConnectionStateEnum.Errored;
		this.error = msg;
		return this;
	}

	private throwTransition(newState: ConnectionStateEnum): never {
		throw Error(
			`Cannot transition this connection from '${ConnectionStateEnum[this.rawState]}' to ${
				ConnectionStateEnum[newState]
			}`
		);
	}
}
