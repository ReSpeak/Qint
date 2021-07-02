import { Writable, writable, get, Readable } from "svelte/store";
import { InBookChangeMsg, WsMessageTarget } from "./backend/ws";
import { Connection } from "./connection";
import {
	binarySearchBy,
	datetimeDeserialize,
	assert,
	factorToDb,
	Cached,
	urlBase64Encode,
} from "./util";
import {
	ChannelGroupId,
	ChannelId,
	ClientId,
	IconId,
	IpAddr,
	ServerGroupId,
	TalkState,
	Uid,
} from "./ts";
import { Codec } from "./book_events";
import * as book_events from "./book_events";
import { Moment } from "moment";
import moment from "moment";
import { ClientBase, ServerBase } from "./bookBase";
import { backend } from "./backend/backend";
import debug from "debug";
const error = debug("error:BOOK");

export function codecToName(codec: Codec): string {
	switch (codec) {
		case Codec.SpeexNarrowband:
			return "Speex Narrowband";
		case Codec.SpeexWideband:
			return "Speex Wideband";
		case Codec.SpeexUltrawideband:
			return "Speex Ultrawideband";
		case Codec.CeltMono:
			return "Celt Mono";
		case Codec.OpusVoice:
			return "Opus Voice";
		case Codec.OpusMusic:
			return "Opus Music";
		default:
			return "Unknown";
	}
}

export class Book {
	public server: Server = new Server();
	public clients: Map<ClientId, Client> = new Map();
	public channels: Map<ChannelId, Channel> = new Map();
	public channelGroups: Writable<Map<ChannelGroupId, Writable<ChannelGroup>>> = writable(
		new Map()
	);
	public serverGroups: Writable<Map<ServerGroupId, Writable<ServerGroup>>> = writable(new Map());
	public currentTalkers: [ClientId, boolean][] = [];
	public ownClientId?: ClientId;
	public ownClient: Writable<Client | undefined> = writable(undefined);

	public reset(): void {
		this.server.reset();
		this.clients.clear();
		this.channels.clear();
		this.channelGroups.set(new Map());
		this.serverGroups.set(new Map());
		this.currentTalkers = [];
		this.ownClientId = undefined;
		this.ownClient.set(undefined);
	}

	public static addChannelSorted(list: Channel[], elem: Channel): Channel[] {
		let start = 0;
		if (elem.order === "0") {
			list.splice(start, 0, elem);
			let lastElem = elem;
			for (let i = start; i < list.length; i++) {
				let c = list[i] as Channel;
				while (c.order === lastElem.id) {
					list.splice(i, 1);
					list.splice(start + 1, 0, c);
					start++;
					lastElem = c;
					i++;
					if (i >= list.length) break;
					c = list[i] as Channel;
				}
			}
		} else {
			let inserted;
			const elems = [elem];
			let lastElem = elem;
			for (let i = start; i < list.length; i++) {
				let c = list[i] as Channel;
				if (inserted === undefined) {
					if (c.id === elem.order) {
						i++;
						list.splice(i, 0, ...elems);
						inserted = i;
					} else {
						while (c.order === lastElem.id) {
							list.splice(i, 1);
							elems.push(c);
							lastElem = c;
							if (i >= list.length) break;
							c = list[i] as Channel;
						}
					}
				} else {
					while (c.order === lastElem.id) {
						list.splice(i, 1);
						list.splice(inserted + 1, 0, c);
						inserted++;
						lastElem = c;
						i++;
						if (i >= list.length) break;
						c = list[i] as Channel;
					}
				}
			}
			if (inserted === undefined) list.splice(list.length, 0, ...elems);
		}
		return list;
	}

	public addChannel(channel: Channel): void {
		if (this.channels.has(channel.id)) throw Error(`Channel ${channel.id} already exists`);
		this.channels.set(channel.id, channel);
		let parent: ITreeParent | undefined;
		if (channel.parent === "0") parent = this.server;
		else parent = this.channels.get(channel.parent);
		if (parent !== undefined) {
			parent.channels.update((pch) => Book.addChannelSorted(pch, channel));
		}
	}

	public updateChannel(
		id: ChannelId,
		obj: Partial<Channel> | Partial<book_events.ChannelGen>
	): void {
		const channel = this.channels.get(id);
		if (channel === undefined) {
			error(`Cannot update non-existant channel ${id}`);
			return;
		}
		const oldParent = channel.parent;
		channel.update(obj as any);
		// Update node in channel tree
		if (channel.parent !== oldParent || "order" in obj) {
			let parent = this.getChannel(oldParent);
			if (parent !== undefined) {
				parent.channels.update((pch) => {
					pch.remove_item(channel);
					return pch;
				});
			}
			parent = this.getChannel(channel.parent);
			if (parent !== undefined) {
				parent.channels.update((pch) => Book.addChannelSorted(pch, channel));
			}
		}
	}

	public removeChannel(id: ChannelId): void {
		const channel = this.getChannel(id);
		if (channel === undefined) return;
		const parent = this.getNode(channel.parent);
		if (parent !== undefined) {
			parent.channels.update((c) => {
				c.remove_item(channel);
				return c;
			});
		}
		this.channels.delete(id);
	}

	private static addClientSorted(list: Client[], elem: Client): Client[] {
		const i = binarySearchBy(list, (t) => {
			const c = t as Client;
			if (elem.talkPower < c.talkPower) return -1;
			if (elem.talkPower > c.talkPower) return 1;
			if (elem.name < c.name) return 1;
			if (elem.name > c.name) return -1;
			return Number(elem.id) - Number(c.id);
		}).index;
		list.splice(i, 0, elem);
		return list;
	}

	public addClient(obj: Partial<Client> | Partial<book_events.ClientGen>): void {
		const client = Client.fromJson(
			obj,
			obj.id === this.ownClientId ? this.ownClient : undefined
		);
		if (this.clients.has(client.id)) throw Error(`Client ${client.id} already exists`);
		this.clients.set(client.id, client);
		const parent = this.channels.get(client.channel);
		if (parent !== undefined) parent.clients.update((pch) => Book.addClientSorted(pch, client));
	}

	public updateClient(id: ClientId, obj: Partial<Client> | Partial<book_events.ClientGen>): void {
		const client = this.getClient(id);
		if (client === undefined) {
			error(`Cannot update non-existant client ${id}`);
			return;
		}
		const oldChannel = client.channel;
		client.update(obj as any);
		// Update node in channel tree
		if (client.channel !== oldChannel || "talkPower" in obj || "name" in obj) {
			let parent = this.getChannel(oldChannel);
			if (parent !== undefined) {
				parent.clients.update((pch) => {
					pch.remove_item(client);
					return pch;
				});
			}
			parent = this.getChannel(client.channel);
			if (parent !== undefined) {
				parent.clients.update((pch) => Book.addClientSorted(pch, client));
			}
		}
	}

	public removeClient(id: ClientId): void {
		const client = this.getClient(id);
		if (client === undefined) return;
		const parent = this.getChannel(client.channel);
		if (parent !== undefined) {
			parent.clients.update((pch) => {
				pch.remove_item(client);
				return pch;
			});
		}
		this.clients.delete(id);
	}

	public addClientServerGroup(id: ClientId, group: ServerGroupId): void {
		const client = this.getClient(id);
		if (client === undefined) {
			error(`Cannot update non-existant client ${id}`);
			return;
		}
		if (!client.serverGroups.includes(group)) {
			client.serverGroups.push(group);
			client.update({}); // TODO nicer?
		}
	}

	public removeClientServerGroup(id: ClientId, group: ServerGroupId): void {
		const client = this.getClient(id);
		if (client === undefined) {
			error(`Cannot update non-existant client ${id}`);
			return;
		}
		client.serverGroups.remove_item(group);
		client.update({}); // TODO nicer?
	}

	public updateServer(obj: Partial<Server> | Partial<book_events.ServerGen>): void {
		this.server.update(obj as any);
	}

	public addServerIp(ip: IpAddr): void {
		this.server.ips.push(ip);
		this.server.update({}); // TODO nicer
	}

	public removeServerIp(ip: IpAddr): void {
		this.server.ips.remove_item(ip);
		this.server.update({}); // TODO nicer
	}

	public addChannelGroup(channelGroup: ChannelGroup): void {
		this.channelGroups.update((channelGroups) => {
			channelGroups.set(channelGroup.id, writable(channelGroup));
			return channelGroups;
		});
	}

	public updateChannelGroup(id: ChannelGroupId, obj: Partial<ChannelGroup>): void {
		const channelGroup = get(this.channelGroups).get(id);
		if (channelGroup === undefined) return;
		channelGroup.update((sg: ChannelGroup) => sg.update(obj));
	}

	public removeChannelGroup(id: ChannelGroupId): void {
		this.channelGroups.update((channelGroups) => {
			channelGroups.delete(id);
			return channelGroups;
		});
	}

	public addServerGroup(serverGroup: ServerGroup): void {
		this.serverGroups.update((serverGroups) => {
			serverGroups.set(serverGroup.id, writable(serverGroup));
			return serverGroups;
		});
	}

	public updateServerGroup(
		id: ServerGroupId,
		obj: Partial<ServerGroup> | Partial<book_events.ServerGroupGen>
	): void {
		const serverGroup = get(this.serverGroups).get(id);
		if (serverGroup === undefined) return;
		serverGroup.update((sg: ServerGroup) => sg.update(obj));
		if ("sortId" in obj) this.serverGroups.update((gs) => gs);
	}

	public removeServerGroup(id: ServerGroupId): void {
		this.serverGroups.update((serverGroups) => {
			serverGroups.delete(id);
			return serverGroups;
		});
	}

	public getNode(id: string): Server | Channel | undefined {
		if (id === "0") return this.server;
		else return this.channels.get(id);
	}

	public getChannel(id: ChannelId): Channel | undefined {
		if (id === "0") return undefined;
		else return this.channels.get(id);
	}

	public getClient(id: ClientId): Client | undefined {
		return this.clients.get(id);
	}

	public getServerGroup(id: ServerGroupId): ServerGroup | undefined {
		const sgStore = get(this.serverGroups).get(id);
		if (sgStore === undefined) return undefined;
		return get(sgStore);
	}

	public sortServerGroupIds(groups: ServerGroupId[]): ServerGroupId[] {
		const grs = [...groups];
		grs.sort((a, b) => {
			const ag = this.getServerGroup(a);
			const bg = this.getServerGroup(b);
			if (ag === undefined || bg === undefined) {
				console.warn("Didn't find server groups", a, b, ag, bg);
				return 0;
			}

			return ag.cmp(bg);
		});
		return grs;
	}

	public messageHandler(msg: InBookChangeMsg): void {
		if ("PropertyAdded" in msg) {
			const id = msg.PropertyAdded.id;
			const prop = msg.PropertyAdded.prop!;
			if ("Channel" in prop) {
				this.addChannel(Channel.fromJson(prop.Channel as any));
			} else if ("OptionalChannelData" in prop && "OptionalChannelData" in id) {
				this.updateChannel(id.OptionalChannelData, {
					optionalData: prop.OptionalChannelData,
				} as any);
			} else if ("ChannelGroup" in prop) {
				this.addChannelGroup(ChannelGroup.fromJson(prop.ChannelGroup));
			} else if ("Client" in prop) {
				this.addClient(prop.Client);
			} else if ("OptionalClientData" in prop && "OptionalClientData" in id) {
				this.updateClient(id.OptionalClientData, {
					optionalData: prop.OptionalClientData,
				} as any);
			} else if ("ConnectionClientData" in prop && "ConnectionClientData" in id) {
				this.updateClient(id.ConnectionClientData, {
					connectionData: prop.ConnectionClientData,
				} as any);
			} else if ("ServerGroupId" in prop && "ClientServerGroup" in id) {
				this.addClientServerGroup(id.ClientServerGroup[0], id.ClientServerGroup[1]);
			} else if ("Server" in prop) {
				this.updateServer(prop.Server);
			} else if ("OptionalServerData" in prop) {
				this.updateServer({ optionalData: prop.OptionalServerData } as any);
			} else if ("ConnectionServerData" in prop) {
				this.updateServer({ connectionData: prop.ConnectionServerData } as any);
			} else if ("IpAddr" in prop && "ServerIp" in id) {
				this.addServerIp(id.ServerIp[0]);
			} else if ("ServerGroup" in prop) {
				this.addServerGroup(ServerGroup.fromJson(prop.ServerGroup));
			}
		} else if ("PropertyChanged" in msg) {
			const id = msg.PropertyChanged.id;
			const prop = msg.PropertyChanged.prop!;
			if ("Channel" in prop && "Channel" in id) {
				this.updateChannel(id.Channel, prop.Channel);
			} else if ("OptionalChannelData" in prop && "OptionalChannelData" in id) {
				this.updateChannel(id.OptionalChannelData, {
					optionalData: prop.OptionalChannelData,
				} as any);
			} else if ("ChannelGroup" in prop && "ChannelGroup" in id) {
				this.updateChannelGroup(id.ChannelGroup, prop.ChannelGroup);
			} else if ("Client" in prop && "Client" in id) {
				this.updateClient(id.Client, prop.Client);
			} else if ("OptionalClientData" in prop && "OptionalClientData" in id) {
				this.updateClient(id.OptionalClientData, {
					optionalData: prop.OptionalClientData,
				} as any);
			} else if ("ConnectionClientData" in prop && "ConnectionClientData" in id) {
				this.updateClient(id.ConnectionClientData, {
					connectionData: prop.ConnectionClientData,
				} as any);
			} else if ("Server" in prop) {
				this.updateServer(prop.Server);
			} else if ("OptionalServerData" in prop) {
				this.updateServer({ optionalData: prop.OptionalServerData } as any);
			} else if ("ConnectionServerData" in prop) {
				this.updateServer({ connectionData: prop.ConnectionServerData } as any);
			} else if ("ServerGroup" in prop && "ServerGroup" in id) {
				this.updateServerGroup(id.ServerGroup, prop.ServerGroup);
			}
		} else if ("PropertyRemoved" in msg) {
			const id = msg.PropertyRemoved.id;
			if ("Channel" in id) {
				this.removeChannel(id.Channel);
			} else if ("OptionalChannelData" in id) {
				this.updateChannel(id.OptionalChannelData, { optionalData: null } as any);
			} else if ("ChannelGroup" in id) {
				this.removeChannelGroup(id.ChannelGroup);
			} else if ("Client" in id) {
				this.removeClient(id.Client);
			} else if ("OptionalClientData" in id) {
				this.updateClient(id.OptionalClientData, { optionalData: null } as any);
			} else if ("ConnectionClientData" in id) {
				this.updateClient(id.ConnectionClientData, { connectionData: null } as any);
			} else if ("ClientServerGroup" in id) {
				this.removeClientServerGroup(id.ClientServerGroup[0], id.ClientServerGroup[1]);
			} else if ("OptionalServerData" in id) {
				this.updateServer({ optionalData: null } as any);
			} else if ("ConnectionServerData" in id) {
				this.updateServer({ connectionData: null } as any);
			} else if ("ServerIp" in id) {
				this.removeServerIp(id.ServerIp[0]);
			} else if ("ServerGroup" in id) {
				this.removeServerGroup(id.ServerGroup);
			}
		}
	}

	public talkersHandler(talkers: [ClientId, boolean][]): void {
		const oldTalkers = this.currentTalkers;
		for (const [id, isWhispering] of talkers) {
			const i = oldTalkers.findIndex((t) => t[0] === id);

			if (i === -1 || oldTalkers[i][1] !== isWhispering) {
				const client = this.getClient(id);
				if (client === undefined) {
					error(`Cannot update non-existant client ${id}`);
					continue;
				}
				client.update({ talking: isWhispering ? TalkState.Whisper : TalkState.Voice });
			}

			if (i !== -1) oldTalkers.splice(i, 1);
		}

		// Remove old talkers
		for (const [id] of oldTalkers) {
			const client = this.getClient(id);
			if (client === undefined) {
				error(`Cannot update non-existant client ${id}`);
				continue;
			}
			client.update({ talking: TalkState.Off });
		}
		this.currentTalkers = talkers;
	}
}

export class ChatData {
	public readonly lastRead: Moment;
	public readonly unreadCount: number;

	public constructor(lastRead: Moment, unreadCount: number) {
		this.lastRead = lastRead;
		this.unreadCount = unreadCount;
	}

	public static fromGraphql(obj: {
		lastRead: number;
		timezone: number;
		unreadCount: number;
	}): ChatData {
		return new ChatData(datetimeDeserialize([obj.lastRead, obj.timezone]), obj.unreadCount);
	}

	public incrementUnread(): ChatData {
		return new ChatData(this.lastRead, this.unreadCount + 1);
	}
}

export class BookNode {
	protected readonly _store: Writable<this>;
	public readonly chat: Writable<ChatData> = writable(new ChatData(moment(), 0));
	public filterShow: boolean = true;
	public isSelected: boolean = false;

	constructor() {
		this._store = writable(this);
	}

	public update(obj: Partial<this>): this {
		Object.assign(this, obj);
		this._store.set(this);
		return this;
	}

	public updateChat(obj: Partial<ChatData>): void {
		this.chat.update((c) => Object.assign(c, obj));
	}

	public subscribe(run: (c: this) => any): () => void {
		return this._store.subscribe(run);
	}
}

export class GraphQlClient extends ClientBase implements ITreeNode {
	public override readonly uid!: Uid | null;
	public override readonly name!: string;
	public override readonly icon!: IconId;
	public override readonly avatarHash!: string;

	protected constructor(uid?: number[], name?: string, icon?: IconId, avatarHash?: string) {
		super();
		// TODO Fix this stupid checks:
		// Either declare the fiels at top null/undefied-able
		// or make sure this is definitely initalized.
		// Otherwise this will be a nasty surprise
		if (name !== undefined) this.name = name;
		if (icon !== undefined) this.icon = icon;
		this.uid = uid ?? null;
		if (avatarHash !== undefined) this.avatarHash = avatarHash;
	}

	public static fromGraphql(obj: {
		uid: number[];
		customName?: string;
		name?: string;
	}): GraphQlClient {
		return new GraphQlClient(obj.uid, obj.customName ?? obj.name, "0", "");
	}

	public static fromGraphqlInvoker(obj: {
		client: { uid: number[]; customName?: string; name?: string };
		icon?: IconId;
		avatar?: string;
	}): GraphQlClient {
		return new GraphQlClient(
			obj.client.uid,
			obj.client.customName ?? obj.client.name,
			obj.icon ?? "0",
			obj.avatar ?? ""
		);
	}

	public readonly qlType = "CLIENT";
	public get qlId(): string {
		return this.uidStr;
	}
	public get wsTarget(): { Client: string } {
		throw "Cannot write to a graphql client";
	}
}

export class Client extends book_events.ClientGen implements ITreeNode, Readable<Client> {
	public volume: Writable<number> = writable(0); // TODO store probably not needed anymore
	public readonly talking: TalkState = TalkState.Off;

	protected constructor() {
		super();
	}

	public static fromJson(
		obj: Partial<Client> | Partial<book_events.ClientGen>,
		store?: Writable<Client | undefined>
	): Client {
		const c = new Client();
		if (store !== undefined) {
			(c._store as any) = store;
		}
		return c.update(obj as any);
	}

	public override update(obj: Partial<this>): this {
		super.update(obj);
		this._store.set(this);
		return this;
	}

	public override equals(other: this): boolean {
		return other instanceof Client && this.id === other.id && super.equals(other);
	}

	public readonly qlType = "CLIENT";
	public get qlId(): string {
		return this.uidStr;
	}
	public get wsTarget(): { Client: string } {
		return { Client: this.id };
	}

	public updateVolume(connection: Connection, volume: number): void {
		assert(this.uid !== null, "Cannot update volume if the client has no uid");
		connection.sendMessage({
			SetClientVolume: {
				client: this.uid,
				volume,
			},
		});
	}

	public async loadVolume(): Promise<void> {
		const res = await backend.graphql(
			`query GetClientVolume($client: [Int!]!) {
			client(uid: $client) { volume }
		}`,
			{
				client: this.uid,
			}
		);
		if (res.data) {
			const volume = res.data.client.volume;
			this.volume.set(factorToDb(volume));
		}
	}
}

export class Channel extends book_events.ChannelGen implements ITreeNode, Readable<Channel> {
	public readonly clients: Writable<Client[]> = writable([]);
	// ITreeParent
	public readonly channels: Writable<Channel[]> = writable([]);
	// Cache last path in file browser
	public lastFilePath: string[] = [];

	protected constructor() {
		super();
	}

	public override update(obj: Partial<this>): this {
		super.update(obj);
		this._store.set(this);
		return this;
	}

	public static fromJson(obj: Partial<Channel> | Partial<book_events.ChannelGen>): Channel {
		return new Channel().update(obj as any);
	}

	public static fromGraphql(obj: Partial<Channel>): Channel {
		return new Channel().update(obj);
	}

	public equals(other: this): boolean {
		return other instanceof Channel && this.id === other.id;
	}

	public readonly qlType = "CHANNEL";
	public get qlId(): string {
		return this.id;
	}
	public readonly wsTarget = "Channel";
}

export class GraphQlServer extends ServerBase implements ITreeNode {
	public override readonly publicKey!: number[];
	public override readonly name!: string;
	public override readonly icon!: IconId;
	public readonly address!: string;

	protected constructor(
		publicKey?: number[] | undefined,
		uid?: number[],
		name?: string,
		address?: string,
		icon?: IconId
	) {
		super(uid);
		if (publicKey !== undefined) this.publicKey = publicKey;
		if (address !== undefined) this.address = address;
		if (name !== undefined) this.name = name;
		if (icon !== undefined) this.icon = icon;
	}

	public static fromGraphql(obj: {
		publicKey?: number[];
		uid?: number[];
		name?: string;
		address?: string;
		icon?: IconId;
	}): GraphQlServer {
		return new GraphQlServer(obj.publicKey, obj.uid, obj.name, obj.address, obj.icon);
	}

	public override equals(other: this): boolean {
		return other instanceof GraphQlServer && this.uidStr === other.uidStr;
	}

	public readonly qlType = "SERVER";
	public get qlId(): string {
		return this.publicKeyStr;
	}
	public readonly wsTarget = "Server";
}

export class Server extends book_events.ServerGen implements ITreeNode, Readable<Server> {
	// ITreeParent
	public readonly channels: Writable<Channel[]> = writable([]);

	constructor() {
		super();
	}

	public override update(obj: Partial<this>): this {
		super.update(obj);
		this._store.set(this);
		return this;
	}

	public reset(): void {
		this.channels.set([]);
		Object.assign(this, { unreadCount: undefined });
		this.filterShow = true;
		this.isSelected = false;
	}

	public readonly qlType = "SERVER";
	public readonly qlId = undefined;
	public readonly wsTarget = "Server";
}

export class ServerGroup extends book_events.ServerGroupGen {
	public static fromJson(
		obj: Partial<ServerGroup> | Partial<book_events.ServerGroupGen>
	): ServerGroup {
		return new ServerGroup().update(obj);
	}

	public cmp(other: ServerGroup): number {
		// If the sortId is 0, the group id is taken
		if (this.id === other.id) return 0;
		const aid = BigInt(this.id);
		const bid = BigInt(other.id);
		const ai = this.sortId === 0 ? aid : BigInt(this.sortId);
		const bi = other.sortId === 0 ? bid : BigInt(other.sortId);
		return ai === bi ? (aid < bid ? -1 : 1) : ai < bi ? -1 : 1;
	}
}

export class ChannelGroup extends book_events.ChannelGroupGen {
	public static fromJson(
		obj: Partial<ChannelGroup> | Partial<book_events.ChannelGroupGen>
	): ChannelGroup {
		return new ChannelGroup().update(obj);
	}
}

export interface ITreeParent {
	channels: Writable<Channel[]>;
}

export type GQLMessageTarget = "SERVER" | "CHANNEL" | "CLIENT" | "POKE";

export interface ITreeNode {
	filterShow: boolean;
	isSelected: boolean;
	readonly chat: Readable<ChatData>;
	updateChat(obj: Partial<ChatData>): void;
	update(obj: Partial<this>): this;
	equals(other: this): boolean;

	readonly qlType: GQLMessageTarget;
	readonly qlId: string | undefined;
	readonly wsTarget: WsMessageTarget;
}
