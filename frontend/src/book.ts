import { Writable, writable, get, Readable } from "svelte/store";
import { InBookChangeMsg, WsMessageTarget } from "./backend/ws";
import { Connection } from "./connection";
import { binarySearchBy,datetimeDeserialize, assert } from "./util";
import { ChannelGroupId, ChannelId, ClientId, IconId, IpAddr, OffsetDateTime, ServerGroupId, TalkState, Uid } from "./ts";
import { Codec } from "./book_events";
import * as book_events from "./book_events";
import { Moment } from "moment";
import moment from "moment";
import { ClientBase, ServerBase } from "./bookBase";
import { backend } from "./backend/backend";

export function codecToName(codec: Codec) {
	switch (codec) {
		case Codec.SpeexNarrowband: return "Speex Narrowband";
		case Codec.SpeexWideband: return "Speex Wideband";
		case Codec.SpeexUltrawideband: return "Speex Ultrawideband";
		case Codec.CeltMono: return "Celt Mono";
		case Codec.OpusVoice: return "Opus Voice";
		case Codec.OpusMusic: return "Opus Music";
		default: return "Unknown";
	}
}

export class Book {
	public server: Server = new Server();
	public clients: Map<ClientId, Client> = new Map();
	public channels: Map<ChannelId, Channel> = new Map();
	public channelGroups: Writable<Map<ChannelGroupId, Writable<ChannelGroup>>> = writable(new Map());
	public serverGroups: Writable<Map<ServerGroupId, Writable<ServerGroup>>> = writable(new Map());
	private currentTalkers: [ClientId, boolean][] = [];
	public ownClientId?: ClientId;
	public ownClient: Writable<Client | undefined> = writable(undefined);

	public reset() {
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
					if (i >= list.length)
						break;
					c = list[i] as Channel;
				}
			}
		} else {
			let inserted;
			let elems = [elem];
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
							if (i >= list.length)
								break;
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
						if (i >= list.length)
							break;
						c = list[i] as Channel;
					}
				}
			}
			if (inserted === undefined)
				list.splice(list.length, 0, ...elems);
		}
		return list;
	}

	public addChannel(channel: Channel) {
		if (this.channels.has(channel.id)) throw Error(`Channel ${channel.id} already exists`);
		this.channels.set(channel.id, channel);
		let parent: ITreeParent | undefined;
		if (channel.parent === "0") parent = this.server;
		else parent = this.channels.get(channel.parent);
		if (parent !== undefined) {
			parent.channels.update(pch => Book.addChannelSorted(pch, channel));
		}
	}

	public updateChannel(id: ChannelId, obj: Partial<Channel>) {
		const channel = this.channels.get(id);
		if (channel === undefined) {
			console.error(`Cannot update non-existant channel ${id}`);
			return;
		}
		const oldParent = channel.parent;
		channel.update(obj);
		// Update node in channel tree
		if (channel.parent !== oldParent || "order" in obj) {
			let parent = this.getChannel(oldParent);
			if (parent !== undefined) {
				parent.channels.update(pch => { pch.remove_item(channel); return pch; });
			}
			parent = this.getChannel(channel.parent);
			if (parent !== undefined) {
				parent.channels.update(pch => Book.addChannelSorted(pch, channel));
			}
		}
	}

	public removeChannel(id: ChannelId): void {
		const channel = this.getChannel(id);
		if (channel === undefined) return;
		const parent = this.getNode(channel.parent);
		if (parent !== undefined) {
			parent.channels.update(c => { c.remove_item(channel); return c; });
		}
		this.channels.delete(id);
	}

	private static addClientSorted(list: Client[], elem: Client): Client[] {
		let i = binarySearchBy(list, t => {
			let c = t as Client;
			if (elem.talkPower < c.talkPower)
				return -1;
			if (elem.talkPower > c.talkPower)
				return 1;
			if (elem.name < c.name)
				return 1;
			if (elem.name > c.name)
				return -1;
			return Number(elem.id) - Number(c.id);
		}).index;
		list.splice(i, 0, elem);
		return list;
	}

	public addClient(obj: Partial<Client>) {
		const client = Client.fromJson(obj, (obj.id === this.ownClientId) ? this.ownClient : undefined);
		if (this.clients.has(client.id)) throw Error(`Client ${client.id} already exists`);
		this.clients.set(client.id, client);
		let parent = this.channels.get(client.channel);
		if (parent !== undefined)
			parent.clients.update(pch => Book.addClientSorted(pch, client));
	}

	public updateClient(id: ClientId, obj: Partial<Client>) {
		const client = this.getClient(id);
		if (client === undefined) {
			console.error(`Cannot update non-existant client ${id}`);
			return;
		}
		const oldChannel = client.channel;
		client.update(obj);
		// Update node in channel tree
		if (client.channel !== oldChannel || "talk_power" in obj || "name" in obj) {
			let parent = this.getChannel(oldChannel);
			if (parent !== undefined) {
				parent.clients.update(pch => { pch.remove_item(client); return pch; });
			}
			parent = this.getChannel(client.channel);
			if (parent !== undefined) {
				parent.clients.update(pch => Book.addClientSorted(pch, client));
			}
		}
	}

	public removeClient(id: ClientId): void {
		const client = this.getClient(id);
		if (client === undefined) return;
		const parent = this.getChannel(client.channel);
		if (parent !== undefined) {
			parent.clients.update(pch => { pch.remove_item(client); return pch; });
		}
		this.clients.delete(id);
	}

	public addClientServerGroup(id: ClientId, group: ServerGroupId) {
		const client = this.getClient(id);
		if (client === undefined) {
			console.error(`Cannot update non-existant client ${id}`);
			return;
		}
		if (!client.serverGroups.includes(group)) {
			client.serverGroups.push(group);
			client.update({}); // TODO nicer?
		}
	}

	public removeClientServerGroup(id: ClientId, group: ServerGroupId) {
		const client = this.getClient(id);
		if (client === undefined) {
			console.error(`Cannot update non-existant client ${id}`);
			return;
		}
		client.serverGroups.remove_item(group);
		client.update({}); // TODO nicer?
	}

	public updateServer(obj: Partial<Server>) {
		this.server.update(obj);
	}

	public addServerIp(ip: IpAddr) {
		this.server.ips.push(ip);
		this.server.update({}); // TODO nicer
	}

	public removeServerIp(ip: IpAddr) {
		this.server.ips.remove_item(ip);
		this.server.update({}); // TODO nicer
	}

	public addChannelGroup(channelGroup: ChannelGroup) {
		this.channelGroups.update(channelGroups => {
			channelGroups.set(channelGroup.id, writable(channelGroup));
			return channelGroups;
		});
	}

	public updateChannelGroup(id: ChannelGroupId, obj: Partial<ChannelGroup>) {
		const channelGroup = get(this.channelGroups).get(id);
		if (channelGroup === undefined)
			return;
		channelGroup.update((sg: ChannelGroup) => sg.update(obj));
	}

	public removeChannelGroup(id: ChannelGroupId) {
		this.channelGroups.update(channelGroups => {
			channelGroups.delete(id);
			return channelGroups;
		});
	}

	public addServerGroup(serverGroup: ServerGroup) {
		this.serverGroups.update(serverGroups => {
			serverGroups.set(serverGroup.id, writable(serverGroup));
			return serverGroups;
		});
	}

	public updateServerGroup(id: ServerGroupId, obj: Partial<ServerGroup>) {
		const serverGroup = get(this.serverGroups).get(id);
		if (serverGroup === undefined)
			return;
		serverGroup.update((sg: ServerGroup) => sg.update(obj));
	}

	public removeServerGroup(id: ServerGroupId) {
		this.serverGroups.update(serverGroups => {
			serverGroups.delete(id);
			return serverGroups;
		});
	}

	public getNode(id: string): Server | Channel | undefined {
		if (id === "0")
			return this.server;
		else
			return this.channels.get(id);
	}

	public getChannel(id: ChannelId): Channel | undefined {
		if (id === "0")
			return undefined;
		else
			return this.channels.get(id);
	}

	public getClient(id: ClientId): Client | undefined {
		return this.clients.get(id);
	}

	public getServerGroup(id: ServerGroupId): ServerGroup | undefined {
		const sgStore = get(this.serverGroups).get(id);
		if (sgStore === undefined) return undefined;
		return get(sgStore);
	}

	public messageHandler(msg: InBookChangeMsg) {
		if ("PropertyAdded" in msg) {
			const prop = msg.PropertyAdded.prop!;
			if ("Channel" in prop) {
				this.addChannel(Channel.fromJson(prop.Channel));
			} else if ("ChannelGroup" in prop) {
				this.addChannelGroup(ChannelGroup.fromJson(prop.ChannelGroup));
			} else if ("Client" in prop) {
				this.addClient(prop.Client);
			} else if ("ServerGroupId" in prop && "ClientServerGroup" in msg.PropertyAdded.id) {
				this.addClientServerGroup(msg.PropertyAdded.id.ClientServerGroup[0],
					msg.PropertyAdded.id.ClientServerGroup[1]);
			} else if ("Server" in prop) {
				this.updateServer(prop.Server);
			} else if ("IpAddr" in prop && "ServerIp" in msg.PropertyAdded.id) {
				this.addServerIp(msg.PropertyAdded.id.ServerIp[0]);
			} else if ("ServerGroup" in prop) {
				this.addServerGroup(ServerGroup.fromJson(prop.ServerGroup));
			}
		} else if ("PropertyChanged" in msg) {
			const prop = msg.PropertyChanged.prop!;
			if ("Channel" in prop && "Channel" in msg.PropertyChanged.id) {
				this.updateChannel(msg.PropertyChanged.id.Channel, prop.Channel);
			} else if ("ChannelGroup" in prop && "ChannelGroup" in msg.PropertyChanged.id) {
				this.updateChannelGroup(msg.PropertyChanged.id.ChannelGroup, prop.ChannelGroup);
			} else if ("Client" in prop && "Client" in msg.PropertyChanged.id) {
				this.updateClient(msg.PropertyChanged.id.Client, prop.Client);
			} else if ("Server" in prop) {
				this.updateServer(prop.Server);
			} else if ("ServerGroup" in prop && "ServerGroup" in msg.PropertyChanged.id) {
				this.updateServerGroup(msg.PropertyChanged.id.ServerGroup, prop.ServerGroup);
			}
		} else if ("PropertyRemoved" in msg) {
			if ("Channel" in msg.PropertyRemoved.id) {
				this.removeChannel(msg.PropertyRemoved.id.Channel);
			} else if ("ChannelGroup" in msg.PropertyRemoved.id) {
				this.removeChannelGroup(msg.PropertyRemoved.id.ChannelGroup);
			} else if ("Client" in msg.PropertyRemoved.id) {
				this.removeClient(msg.PropertyRemoved.id.Client);
			} else if ("ClientServerGroup" in msg.PropertyRemoved.id) {
				this.removeClientServerGroup(msg.PropertyRemoved.id.ClientServerGroup[0],
					msg.PropertyRemoved.id.ClientServerGroup[1]);
			} else if ("ServerIp" in msg.PropertyRemoved.id) {
				this.removeServerIp(msg.PropertyRemoved.id.ServerIp[0]);
			} else if ("ServerGroup" in msg.PropertyRemoved.id) {
				this.removeServerGroup(msg.PropertyRemoved.id.ServerGroup);
			}
		}
	}

	public talkersHandler(talkers: [ClientId, boolean][]) {
		let oldTalkers = this.currentTalkers;
		for (const [id, isWhispering] of talkers) {
			const i = oldTalkers.findIndex(t => t[0] === id);

			if (i === -1 || oldTalkers[i][1] !== isWhispering) {
				const client = this.getClient(id);
				if (client === undefined) {
					console.error(`Cannot update non-existant client ${id}`);
					continue;
				}
				client.update({ talking: isWhispering ? TalkState.Whisper : TalkState.Voice });
			}

			if (i !== -1)
				oldTalkers.splice(i, 1);
		}

		// Remove old talkers
		for (const [id,] of oldTalkers) {
			const client = this.getClient(id);
			if (client === undefined) {
				console.error(`Cannot update non-existant client ${id}`);
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

	public static fromGraphql(obj: any): ChatData {
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

	public updateChat(obj: Partial<ChatData>) {
		this.chat.update(c => Object.assign(c, obj));
	}

	public subscribe(run: (c: this) => any): () => void {
		return this._store.subscribe(run);
	}
}

export class GraphQlClient extends ClientBase {
	public readonly uid!: Uid | null;
	public readonly name!: string;
	public readonly icon!: IconId;

	protected constructor(uid?: number[], name?: string, icon?: IconId, avatar_hash?: string) {
		super(uid, avatar_hash);
		if (name !== undefined) this.name = name;
		if (icon !== undefined) this.icon = icon;
	}

	public static fromGraphqlInvoker(obj: any): GraphQlClient {
		return new GraphQlClient(obj.client.uid, obj.client.customName ?? obj.client.name, obj.icon ?? 0, obj.avatar ?? "");
	}
}

export class Client extends book_events.Client implements ITreeNode, Readable<Client> {
	public volume: Writable<number> = writable(0); // TODO store probably not needed anymore
	public readonly talking: TalkState = TalkState.Off;

	protected constructor() {
		super();
	}

	public static fromJson(obj: Partial<Client>, store?: Writable<Client | undefined>): Client {
		let c = new Client();
		if (store !== undefined) {
			(c._store as any) = store;
		}
		return c.update(obj);
	}

	public update(obj: Partial<this>): this {
		super.update(obj);
		this._store.set(this);
		return this;
	}

	public equals(other: this): boolean {
		return other instanceof Client && this.id === other.id && super.equals(other);
	}

	public readonly qlType = "CLIENT";
	public get qlId() { return this.uidStr; }
	public get wsTarget() { return { Client: this.id }; }

	public updateVolume(connection: Connection, volume: number) {
		assert(this.uid !== null, "Cannot update volume if the client has no uid");
		connection.sendMessage({
			SetClientVolume: {
				client: this.uid,
				volume,
			},
		});
	}

	public async loadVolume() {
		const res = await backend.graphql(`query GetClientVolume($client: [Int!]!) {
			client(uid: $client) { volume }
		}`, {
			client: this.uid,
		});
		if (res.data) {
			const volume = res.data.client.volume;
			// TODO Have a constant for the minimum volume
			const scaledVol: number = volume === 0 ? -30 : Math.round(20 * Math.log10(volume));
			this.volume.update(() => scaledVol);
		}
	}
}

export class Channel extends book_events.Channel implements ITreeNode, Readable<Channel> {
	public readonly clients: Writable<Client[]> = writable([]);
	// ITreeParent
	public readonly channels: Writable<Channel[]> = writable([]);

	protected constructor() {
		super();
	}

	public update(obj: Partial<this>): this {
		super.update(obj);
		this._store.set(this);
		return this;
	}

	public static fromJson(obj: Partial<Channel>): Channel {
		return new Channel().update(obj);
	}

	public static fromGraphql(obj: any): Channel {
		return new Channel().update(obj);
	}

	public equals(other: this): boolean {
		return other instanceof Channel && this.id === other.id;
	}

	public readonly qlType = "CHANNEL";
	public get qlId() { return this.id };
	public readonly wsTarget = "Channel";
}

export class GraphQlServer extends ServerBase {
	public readonly public_key!: number[];
	public readonly name!: string;
	public readonly icon!: IconId;

	protected constructor(public_key?: number[] | undefined, uid?: number[], name?: string, icon?: IconId) {
		super(uid);
		if (public_key !== undefined) this.public_key = public_key;
		if (name !== undefined) this.name = name;
		if (icon !== undefined) this.icon = icon;
	}

	public static fromGraphql(obj: any): GraphQlServer {
		return new GraphQlServer(obj.server.publicKey, obj.server.uid, obj.server.name, obj.server.icon);
	}

	public equals(other: this): boolean {
		return other instanceof GraphQlServer && this.uidStr === other.uidStr;
	}
}

export class Server extends book_events.Server implements ITreeNode, Readable<Server> {
	// ITreeParent
	public readonly channels: Writable<Channel[]> = writable([]);

	constructor() {
		super();
	}

	public update(obj: Partial<this>): this {
		super.update(obj);
		this._store.set(this);
		return this;
	}

	public reset() {
		this.channels.set([]);
		Object.assign(this, { unreadCount: undefined });
		this.filterShow = true;
		this.isSelected = false;
	}

	public readonly qlType = "SERVER";
	public readonly qlId = undefined;
	public readonly wsTarget = "Server";
}

export class OldServer extends GraphQlServer implements ITreeParent, ITreeNode, Readable<OldServer> {
	public readonly phonetic_name!: string;
	public readonly ips!: string[];
	public readonly license!: string; // TODO enum
	public readonly created!: OffsetDateTime;
	public readonly max_clients!: number;
	public readonly nickname!: string;
	public readonly platform!: string;
	public readonly version!: string;
	public readonly welcome_message!: string;

	constructor() {
		super();
	}
	// ITreeParent
	public channels: Writable<Channel[]> = writable([]);

	public reset() {
		this.channels.set([]);
		Object.assign(this, { unreadCount: undefined });
		this.filterShow = true;
		this.isSelected = false;
	}

	public readonly qlType = "SERVER";
	public readonly qlId = undefined;
	public readonly wsTarget = "Server";
}

export class ServerGroup extends book_events.ServerGroup {
	public static fromJson(obj: Partial<ServerGroup>): ServerGroup {
		return new ServerGroup().update(obj);
	}

	public cmp(other: ServerGroup): number {
		// If the sortId is 0, the group id is taken
		const ai = this.sortId === 0 ? BigInt(this.id) : BigInt(this.sortId);
		const bi = other.sortId === 0 ? BigInt(other.id) : BigInt(other.sortId);
		return ai === bi ? 0 : (ai < bi ? -1 : 1);
	}
}

export class ChannelGroup extends book_events.ChannelGroup {
	public static fromJson(obj: Partial<ChannelGroup>): ChannelGroup {
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
