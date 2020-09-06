import { Writable, writable, get } from "svelte/store";
import { InBookChangeMsg } from "../structs/ws";
import { graphql } from "../graphql";
import { Connection } from "../connection";
import { binarySearchBy, binarySearchByKey, getDataColor, arraysEqual, Lazy } from "../util";
import "../extensions";
import { ChannelId, ChannelType, ClientId, Codec, ServerGroupId } from "../structs/ts";

export class Book {
	public server: Writable<Server> = writable(new Server());
	public clients: Writable<Map<number, Client>> = writable(new Map());
	public channels: Writable<Map<number, Channel>> = writable(new Map());
	public channelGroups: Writable<Map<number, Writable<ChannelGroup>>> = writable(new Map());
	public serverGroups: Writable<Map<number, Writable<ServerGroup>>> = writable(new Map());
	private currentTalkers: [number, boolean][] = [];

	public reset() {
		this.server.set(new Server());
		this.clients.set(new Map());
		this.channels.set(new Map());
		this.currentTalkers = [];
	}

	private static findChannelStart(list: ArrayLike<ITreeNode>): number {
		return binarySearchByKey(list, 1, e => e instanceof Client ? 0 : 2).index;
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

	private static addChannelSorted(list: ITreeNode[], elem: Channel): ITreeNode[] {
		let start = Book.findChannelStart(list);
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
		this.channels.update(channels => {
			if (channels.has(channel.id)) throw Error(`Channel ${channel.id} already exists`);
			channels.set(channel.id, channel);
			let parent: ITreeParent | undefined;
			if (channel.parent === 0) parent = get(this.server);
			else parent = channels.get(channel.parent);
			if (parent) {
				parent.children.update(pch => Book.addChannelSorted(pch, channel));
			}
			return channels;
		});
	}

	public updateChannel(id: number, obj: Partial<Channel>) {
		this.channels.update(channels => {
			const channel = channels.get(id);
			if (channel === undefined) {
				console.error(`Cannot update non-existant channel ${id}`);
				return channels;
			}
			const oldParent = channel.parent;
			channel.update(obj);
			// Update node in channel tree
			if ("parent" in obj || "order" in obj) {
				let parent = this.getChannel(oldParent);
				if (parent !== undefined) {
					parent.children.update(pch => { pch.remove_item(channel); return pch; });
				}
				parent = this.getChannel(channel.parent);
				if (parent !== undefined) {
					parent.children.update(pch => Book.addChannelSorted(pch, channel));
				}
			}
			return channels;
		});
	}

	public removeChannel(id: number): void {
		const channel = this.getChannel(id);
		if (channel === undefined) return;
		const parent = this.getNode(channel.parent);
		if (parent !== undefined) {
			parent.children.update(c => { c.remove_item(channel); return c; });
		}
		this.channels.update(channels => {
			channels.delete(id);
			return channels;
		});
	}

	private static addClientSorted(list: ITreeNode[], elem: Client): ITreeNode[] {
		let end = Book.findChannelStart(list);
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
		}, 0, end).index;
		list.splice(i, 0, elem);
		return list;
	}

	public addClient(client: Client) {
		this.clients.update(clients => {
			if (clients.has(client.id)) throw Error(`Client ${client.id} already exists`);
			clients.set(client.id, client);
			let parent: ITreeParent | undefined = get(this.channels).get(client.channel);
			if (parent !== undefined)
				parent.children.update(pch => Book.addClientSorted(pch, client));
			return clients;
		});
	}

	public updateClient(id: ClientId, obj: Partial<Client>) {
		this.clients.update(clients => {
			const client = this.getClient(id);
			if (client === undefined) {
				console.error(`Cannot update non-existant client ${id}`);
				return clients;
			}
			const oldChannel = client.channel;
			client.update(obj);
			// Update node in channel tree
			if ("channel" in obj || "talk_power" in obj || "name" in obj) {
				let parent = this.getChannel(oldChannel);
				if (parent !== undefined) {
					parent.children.update(pch => { pch.remove_item(client); return pch; });
				}
				parent = this.getChannel(client.channel);
				if (parent !== undefined) {
					parent.children.update(pch => Book.addClientSorted(pch, client));
				}
			} else {
				const parent = this.getChannel(client.channel);
				if (parent !== undefined) {
					parent.children.update(pch => pch);
				}
			}
			return clients;
		});
	}

	public removeClient(id: ClientId): void {
		const client = this.getClient(id);
		if (client === undefined) return;
		const parent = this.getChannel(client.channel);
		if (parent !== undefined) {
			parent.children.update(pch => { pch.remove_item(client); return pch; });
		}
		this.clients.update(clients => {
			clients.delete(id);
			return clients;
		});
	}

	public addClientServerGroup(id: ClientId, group: ServerGroupId) {
		this.clients.update(clients => {
			const client = this.getClient(id);
			if (client === undefined) {
				console.error(`Cannot update non-existant client ${id}`);
				return clients;
			}
			client.server_groups.push(group);
			return clients;
		});
	}

	public removeClientServerGroup(id: ClientId, group: ServerGroupId) {
		this.clients.update(clients => {
			const client = this.getClient(id);
			if (client === undefined) {
				console.error(`Cannot update non-existant client ${id}`);
				return clients;
			}
			client.server_groups.remove_item(group);
			return clients;
		});
	}

	public updateServer(obj: Partial<Server>) {
		this.server.update(s => {
			s.update(obj);
			return s;
		});
	}

	public addServerIp(ip: string) {
		this.server.update(s => {
			s.ips.push(ip);
			return s;
		})
	}

	public removeServerIp(ip: string) {
		this.server.update(s => {
			s.ips.remove_item(ip);
			return s;
		})
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

	public getServer(): Server {
		return get(this.server);
	}

	public getNode(id: number): Server | Channel | undefined {
		if (id === 0)
			return get(this.server);
		else
			return get(this.channels).get(id);
	}

	public getChannel(id: number): Channel | undefined {
		if (id === 0)
			return;
		else
			return get(this.channels).get(id);
	}

	public getClient(id: number): Client | undefined {
		return get(this.clients).get(id);
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
				this.addClient(Client.fromJson(prop.Client));
			} else if ("ServerGroupId" in prop && "ClientServerGroup" in msg.PropertyAdded.id) {
				this.addClientServerGroup(msg.PropertyAdded.id.ClientServerGroup[0],
					msg.PropertyAdded.id.ClientServerGroup[1]);
			} else if ("Server" in prop) {
				this.updateServer(prop.Server);
				document.title = get(this.server).name + " – Qint";
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
		for (var talker of talkers) {
			const id = talker[0];
			const i = oldTalkers.findIndex(t => t[0] === id);

			if (i === -1 || oldTalkers[i][1] !== talker[1]) {
				this.clients.update(clients => {
					const client = this.getClient(id);
					if (client === undefined) {
						console.error(`Cannot update non-existant client ${id}`);
						return clients;
					}
					client.talking = talker[1];
					const parent = this.getChannel(client.channel);
					if (parent !== undefined) {
						parent.children.update(pch => pch);
					}
					return clients;
				});
			}

			if (i !== -1)
				oldTalkers.splice(i, 1);
		}

		// Remove old talkers
		for (var talker of oldTalkers) {
			const id = talker[0];
			this.clients.update(clients => {
				const client = this.getClient(id);
				if (client === undefined) {
					console.error(`Cannot update non-existant client ${id}`);
					return clients;
				}
				client.talking = undefined;
				const parent = this.getChannel(client.channel);
				if (parent !== undefined) {
					parent.children.update(pch => pch);
				}
				return clients;
			});
		}
		this.currentTalkers = talkers;
	}
}

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
		c.uid = [];
		const b = atob(obj.client.uid);
		for (let i = 0; i < b.length; i++) {
			c.uid.push(b.charCodeAt(i));
		}
		c.name = obj.client.customName ?? obj.client.name;
		c.icon_id = obj.icon ?? 0;
		c.avatar_hash = obj.avatar ?? "";
		return c;
	}

	private getUid(): string {
		let res = "";
		for (let i = 0; i < this.uid.length; i++) {
			res += String.fromCharCode(this.uid[i]);
		}
		return btoa(res);
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
		let res = "";
		for (let i = 0; i < this.uid.length; i++) {
			const c = this.uid[i];
			res += String.fromCharCode('a'.charCodeAt(0) + (c >> 4));
			res += String.fromCharCode('a'.charCodeAt(0) + (c & 0xf));
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

export class Client extends GraphQlClient implements ITreeNode {
	public avatar_hash!: string;
	public away_message!: string | null;
	public badges!: string;
	public channel!: number;
	public channel_group!: number;
	public server_groups!: number[];
	public client_type!: string;
	public country_code!: string;
	public database_id!: number;
	public description!: string
	//public icon_id!: IconHash; // inherited from GraphQlClient
	public id!: number;
	public inherited_channel_group_from_channel!: number;
	public input_hardware_enabled!: boolean;
	public input_muted!: boolean;
	public is_channel_commander!: boolean;
	public is_priority_speaker!: boolean;
	public is_recording!: boolean;
	public metadata!: string
	public name!: string;
	public needed_serverquery_view_power!: number;
	public output_hardware_enabled!: boolean;
	public output_muted!: boolean;
	public output_only_muted!: boolean;
	public permission_hints!: string | null;
	public phonetic_name!: string;
	public talk_power!: number;
	public talk_power_granted!: boolean;
	public talk_power_request!: string | null;
	public unread_messages!: number;

	public volume: Writable<number> = writable(0);
	/// true if whispering, false if talking, undefined if silent
	public talking?: boolean;

	// ITreeParent
	public children: Writable<ITreeNode[]> = writable([]);

	// ITeeeNode
	public filterShow: boolean = true;
	public get key() { return `u${this.id}`; }

	protected constructor() { super(); }

	public static fromJson(obj: Partial<Client>): Client {
		return new Client().update(obj);
	}

	public update(obj: Partial<this>): this {
		return Object.assign(this, obj);
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
			connection: connection.guid,
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

type MaxClients = "Inherited" | "Unlimited" | { Limited: number };

export class Channel implements ITreeParent, ITreeNode {
	public id!: ChannelId;
	public parent!: ChannelId;
	public name!: string;
	public topic!: string | null;
	public codec!: Codec;
	public codec_quality!: number | null;
	public max_clients!: MaxClients;
	public max_family_clients!: MaxClients | null;
	public order!: ChannelId;
	public channel_type!: ChannelType; // Why is this called 'channel_' ?
	public is_default!: boolean | null;
	public has_password!: boolean | null;
	public codec_latency_factor!: number | null;
	public is_unencrypted!: boolean | null;
	public delete_delay!: any | null;
	public needed_talk_power!: number | null;
	public forced_silence!: boolean | null;
	public phonetic_name!: string | null;
	public icon_id!: IconHash;
	public is_private!: boolean | null;
	public subscribed!: boolean;
	public permission_hints!: any | null;
	public optional_data!: any | null;

	// ITreeParent
	public children: Writable<ITreeNode[]> = writable([]);

	// ITeeeNode
	public filterShow: boolean = true;
	public get key() { return `c${this.id}`; }

	private constructor() { }

	public static fromJson(obj: Partial<Channel>): Channel {
		return new Channel().update(obj);
	}

	public update(obj: Partial<this>): this {
		return Object.assign(this, obj);
	}
}

export class Server implements ITreeParent {
	public name!: string;
	public phonetic_name!: string;
	public icon_id!: IconHash;
	public public_key?: number[];
	// Base64 encoded, result from graphql
	public publicKey?: string;
	public ips!: string[];

	// ITreeParent
	public children: Writable<ITreeNode[]> = writable([]);

	public update(obj: Partial<this>): this {
		return Object.assign(this, obj);
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

type GroupNamingMode = any;
type IconHash = number | undefined;
type GroupType = any;

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
	children: Writable<ITreeNode[]>;
}

export interface ITreeNode {
	filterShow: boolean;
	key: any;
}
