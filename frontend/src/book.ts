import { Writable, writable, get, Readable } from "svelte/store";
import { InBookChangeMsg } from "./backend/ws";
import { graphql } from "./graphql";
import { Connection } from "./connection";
import { binarySearchBy, getDataColor, arraysEqual, Lazy, base64Decode, base64Encode } from "./util";
import { ChannelId, ChannelType, ClientId, Codec, ServerGroupId } from "./ts";

export class Book {
	public server: Server = new Server();
	public clients: Map<number, Client> = new Map();
	public channels: Map<number, Channel> = new Map();
	public channelGroups: Writable<Map<number, Writable<ChannelGroup>>> = writable(new Map());
	public serverGroups: Writable<Map<number, Writable<ServerGroup>>> = writable(new Map());
	private currentTalkers: [number, boolean][] = [];
	public ownClientId?: number;
	public ownClient: Writable<Client | undefined> = writable(undefined);

	public reset() {
		this.server = new Server();
		this.clients.clear();
		this.channels.clear();
		this.channelGroups.set(new Map());
		this.serverGroups.set(new Map());
		this.currentTalkers = [];
		this.ownClientId = undefined;
		this.ownClient.set(undefined);
	}

	private static listString(list: ITreeNode[]): string {
		let res = "";
		for (let e of list) {
			if (e instanceof Client) {
				res += `${e.name}, `;
			} else if (e instanceof Channel) {
				res += `(${e.name}, ${e.id}, ${e.order}), `;
			} else {
				res += `${e}, `;
			}
		}
		return res;
	}

	public static addChannelSorted(list: Channel[], elem: Channel): Channel[] {
		let start = 0;
		if (elem.order === 0) {
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
		if (channel.parent === 0) parent = this.server;
		else parent = this.channels.get(channel.parent);
		if (parent !== undefined) {
			parent.channels.update(pch => Book.addChannelSorted(pch, channel));
		}
	}

	public updateChannel(id: number, obj: Partial<Channel>) {
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

	public removeChannel(id: number): void {
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
			if (elem.talk_power < c.talk_power)
				return -1;
			if (elem.talk_power > c.talk_power)
				return 1;
			if (elem.name < c.name)
				return 1;
			if (elem.name > c.name)
				return -1;
			return elem.id - c.id;
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
		if (!client.server_groups.includes(group)) {
			client.server_groups.push(group);
			client.update({}); // TODO nicer?
		}
	}

	public removeClientServerGroup(id: ClientId, group: ServerGroupId) {
		const client = this.getClient(id);
		if (client === undefined) {
			console.error(`Cannot update non-existant client ${id}`);
			return;
		}
		client.server_groups.remove_item(group);
		client.update({}); // TODO nicer?
	}

	public updateServer(obj: Partial<Server>) {
		this.server.update(obj);
	}

	public addServerIp(ip: string) {
		this.server.ips.push(ip);
		this.server.update({}); // TODO nicer
	}

	public removeServerIp(ip: string) {
		this.server.ips.remove_item(ip);
		this.server.update({}); // TODO nicer
	}

	public addChannelGroup(channelGroup: ChannelGroup) {
		this.channelGroups.update(channelGroups => {
			channelGroups.set(channelGroup.id, writable(channelGroup));
			return channelGroups;
		});
	}

	public updateChannelGroup(id: number, obj: Partial<ChannelGroup>) {
		const channelGroup = get(this.channelGroups).get(id);
		if (channelGroup === undefined)
			return;
		channelGroup.update((sg: ChannelGroup) => sg.update(obj));
	}

	public removeChannelGroup(id: number) {
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

	public updateServerGroup(id: number, obj: Partial<ServerGroup>) {
		const serverGroup = get(this.serverGroups).get(id);
		if (serverGroup === undefined)
			return;
		serverGroup.update((sg: ServerGroup) => sg.update(obj));
	}

	public removeServerGroup(id: number) {
		this.serverGroups.update(serverGroups => {
			serverGroups.delete(id);
			return serverGroups;
		});
	}

	public getNode(id: number): Server | Channel | undefined {
		if (id === 0)
			return this.server;
		else
			return this.channels.get(id);
	}

	public getChannel(id: number): Channel | undefined {
		if (id === 0)
			return undefined;
		else
			return this.channels.get(id);
	}

	public getClient(id: number): Client | undefined {
		return this.clients.get(id);
	}

	public getServerGroup(id: number): ServerGroup | undefined {
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

	public talkersHandler(talkers: [number, boolean][]) {
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

type MaxClients = "Inherited" | "Unlimited" | { Limited: number };
type GroupNamingMode = any;
type IconHash = number | undefined;
type GroupType = any;
export type OffsetDateTime = [number, number];


export class GraphQlClient {
	public uid!: number[];
	public name!: string;
	public icon_id!: IconHash;
	public avatar_hash!: string;
	private _clientColor: Lazy<string>;
	public get clientColor() { return this._clientColor.get(); }
	private _uidStr: Lazy<string>;
	public get uidStr() { return this._uidStr.get(); }

	protected constructor() {
		this._clientColor = new Lazy(() => getDataColor(this.uid));
		this._uidStr = new Lazy(() => this.getUid());
	}

	public static fromGraphqlInvoker(obj: any): GraphQlClient {
		const c = new GraphQlClient();
		c.uid = base64Decode(obj.client.uid);
		c.name = obj.client.customName ?? obj.client.name;
		c.icon_id = obj.icon ?? 0;
		c.avatar_hash = obj.avatar ?? "";
		return c;
	}

	private getUid(): string {
		return base64Encode(this.uid);
	}

	/**
	 * TeamSpeak uses a different encoding of the uid for fetching avatars.
	 *
	 * The raw data (base64-decoded) is encoded in hex, but instead of using
	 * [0-9a-f] with [a-p].
	 */
	public getAvatarUid(): string | undefined {
		if (this.avatar_hash === "")
			return;
		const a0 = 'a'.charCodeAt(0);
		let res = "";
		for (let i = 0; i < this.uid.length; i++) {
			const c = this.uid[i];
			res += String.fromCharCode(a0 + (c >> 4));
			res += String.fromCharCode(a0 + (c & 0xf));
		}
		return res;
	}

	public equals(other: GraphQlClient | undefined): boolean { return GraphQlClient.equals(this, other); }

	public static equals(first: GraphQlClient | undefined, second: GraphQlClient | undefined): boolean {
		if (first === second) return true;
		if (first === undefined || second === undefined) return false;
		return arraysEqual(first.uid, second.uid);
	}
}

export class Client extends GraphQlClient implements ITreeNode, Readable<Client> {
	private _store: Writable<this>;
	public readonly avatar_hash!: string;
	public readonly away_message!: string | null;
	public readonly badges!: string;
	public readonly channel!: number;
	public readonly channel_group!: number;
	public readonly server_groups!: number[];
	public readonly client_type!: string;
	public readonly country_code!: string;
	public readonly database_id!: number;
	public readonly description!: string
	//public readonly icon_id!: IconHash; // inherited from GraphQlClient
	public readonly id!: number;
	public readonly inherited_channel_group_from_channel!: number;
	public readonly input_hardware_enabled!: boolean;
	public readonly input_muted!: boolean;
	public readonly is_channel_commander!: boolean;
	public readonly is_priority_speaker!: boolean;
	public readonly is_recording!: boolean;
	public readonly metadata!: string
	public readonly name!: string;
	public readonly needed_serverquery_view_power!: number;
	public readonly output_hardware_enabled!: boolean;
	public readonly output_muted!: boolean;
	public readonly output_only_muted!: boolean;
	public readonly permission_hints!: string | null;
	public readonly phonetic_name!: string;
	public readonly talk_power!: number;
	public readonly talk_power_granted!: boolean;
	public readonly talk_power_request!: string | null;
	public readonly unread_messages!: number;

	public volume: Writable<number> = writable(0); // TODO store probably not needed anymore
	public readonly talking: TalkState = TalkState.Off;

	// ITreeNode
	public filterShow: boolean = true;
	public isSelected: boolean = false;

	protected constructor(store?: Writable<Client | undefined>) {
		super();
		this._store = (store ?? writable(undefined)) as Writable<this>;
	}

	public static fromJson(obj: Partial<Client>, store?: Writable<Client | undefined>): Client {
		return new Client(store).update(obj);
	}

	public update(obj: Partial<this>): this {
		Object.assign(this, obj);
		this._store.set(this);
		return this;
	}

	public subscribe(run: (c: this) => any): () => void {
		return this._store.subscribe(run);
	}

	public getColor() {
		if (this.uid) {
			return getDataColor(this.uid)
		} else {
			return getDataColor(this.name);
		}
	}

	public async updateVolume(connection: Connection, volume: number): Promise<void> {
		await graphql(`mutation SetClientVolume($connection: ID!, $client: ID!, $volume: Float!) {
			setClientVolume(connection: $connection, client: $client, volume: $volume) { void }
		}`, {
			connection: connection.backend?.getGuidTmpHack(),
			client: this.uidStr,
			volume,
		});
	}

	public async loadVolume() {
		const res = await graphql(`query GetClientVolume($client: ID!) {
			client(uid: $client) { volume }
		}`, {
			client: this.uidStr,
		});
		if (res.data) {
			const volume = res.data.client.volume;
			const scaledVol: number = volume === 0 ? 0 : Math.round(20 * Math.log10(volume));
			this.volume.update(() => scaledVol);
		}
	}
}

export enum TalkState {
	Off,
	Voice,
	Whisper
}

export class Channel implements ITreeParent, ITreeNode, Readable<Channel> {
	public _store: Writable<this>;
	public readonly id!: ChannelId;
	public readonly parent!: ChannelId;
	public readonly name!: string;
	public readonly topic!: string | null;
	public readonly codec!: Codec;
	public readonly codec_quality!: number | null;
	public readonly max_clients!: MaxClients;
	public readonly max_family_clients!: MaxClients | null;
	public readonly order!: ChannelId;
	public readonly channel_type!: ChannelType; // Why is this called 'channel_' ?
	public readonly is_default!: boolean | null;
	public readonly has_password!: boolean | null;
	public readonly codec_latency_factor!: number | null;
	public readonly is_unencrypted!: boolean | null;
	public readonly delete_delay!: any | null;
	public readonly needed_talk_power!: number | null;
	public readonly forced_silence!: boolean | null;
	public readonly phonetic_name!: string | null;
	public readonly icon_id!: IconHash;
	public readonly is_private!: boolean | null;
	public readonly subscribed!: boolean;
	public readonly permission_hints!: any | null;
	public readonly optional_data!: any | null;

	public readonly clients: Writable<Client[]> = writable([]);
	// ITreeParent
	public readonly channels: Writable<Channel[]> = writable([]);

	// ITreeNode
	public filterShow: boolean = true;
	public isSelected: boolean = false;

	private constructor() {
		this._store = writable(this);
	}

	public static fromJson(obj: Partial<Channel>): Channel {
		return new Channel().update(obj);
	}

	public static fromGraphql(obj: any): Channel {
		return new Channel().update({
			id: obj.id,
			parent: obj.parent,
			name: obj.name,
			order: obj.orderId,
			icon_id: obj.icon
		});
	}

	public update(obj: Partial<this>): this {
		Object.assign(this, obj);
		this._store.set(this);
		return this;
	}

	public subscribe(run: (c: this) => any): () => void {
		return this._store.subscribe(run);
	}
}

export class Server implements ITreeParent, ITreeNode, Readable<Server> {
	public _store: Writable<this>;
	public readonly name!: string;
	public readonly phonetic_name!: string;
	public readonly icon_id!: IconHash;
	public readonly public_key?: number[];
	// Base64 encoded, result from graphql
	public readonly publicKey?: string;
	public readonly ips!: string[];
	public readonly license!: string; // TODO enum
	public readonly created!: OffsetDateTime;
	public readonly max_clients!: number;
	public readonly nickname!: string;
	public readonly platform!: string;
	public readonly version!: string;
	public readonly welcome_message!: string;

	public filterShow: boolean = true;
	public isSelected: boolean = false;

	constructor() {
		this._store = writable(this);
	}

	// ITreeParent
	public channels: Writable<Channel[]> = writable([]);

	public update(obj: Partial<this>): this {
		Object.assign(this, obj);
		this._store.set(this);
		return this;
	}

	public subscribe(run: (c: this) => any): () => void {
		return this._store.subscribe(run);
	}

	public getColor() {
		if (this.public_key) {
			return getDataColor(this.public_key)
		} else if (this.publicKey) {
			return getDataColor(atob(this.publicKey))
		} else {
			return getDataColor(this.name ?? "");
		}
	}
}

export class Group {
	id!: number;
	name!: string;
	group_type!: GroupType;
	icon_id!: IconHash;
	is_permanent!: boolean;
	sort_id!: number;
	naming_mode!: GroupNamingMode;
	needed_modify_power!: number;
	needed_member_add_power!: number;
	needed_member_remove_power!: number;

	public update(obj: Partial<this>): this {
		return Object.assign(this, obj);
	}
}

export class ServerGroup extends Group {
	public static fromJson(obj: Partial<ServerGroup>): ServerGroup {
		return new ServerGroup().update(obj);
	}
};
export class ChannelGroup extends Group {
	public static fromJson(obj: Partial<ChannelGroup>): ChannelGroup {
		return new ChannelGroup().update(obj);
	}
};

export interface ITreeParent {
	channels: Writable<Channel[]>;
}

export interface ITreeNode {
	filterShow: boolean;
	isSelected: boolean;
	update(obj: Partial<this>): this;
}
