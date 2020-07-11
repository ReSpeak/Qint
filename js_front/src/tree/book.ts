import { Writable, writable, Readable, derived, get } from "svelte/store";
import { InBookChangeMsg } from "../structs/ws";
import { graphql } from "../graphql";
import { Connection } from "../connection";
import { binarySearchBy, binarySearchByKey, getDataColor, arraysEqual, IArray } from "../util";
import "../extensions";

type ChannelId = number;

export class Book {
	public server: Writable<Server> = writable(new Server());
	public clients: Writable<Map<number, Client>> = writable(new Map());
	public channels: Writable<Map<number, Channel>> = writable(new Map());
	private currentTalkers: [number, boolean][] = [];

	public reset() {
		this.server.set(new Server());
		this.clients.set(new Map());
		this.channels.set(new Map());
		this.currentTalkers = [];
	}

	private static findChannelStart(list: IArray<ITreeNode>): number {
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
				console.log(e);
				res += `${e}, `;
			}
		}
		return res;
	}

	private static addChannelSorted(list: ITreeNode[], elem: Channel): ITreeNode[] {
		//console.log("before", Book.listString(list));
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
		//console.log("after", Book.listString(list));
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

	public updateChannel(id: number, obj: any) {
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
		//console.log("before", Book.listString(list));
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
		//console.log("after", Book.listString(list));
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

	public updateClient(id: number, obj: any) {
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

	public removeClient(id: number): void {
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

	public updateServer(obj: any) {
		this.server.update(s => {
			s.update(obj);
			return s;
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

	public messageHandler(msg: InBookChangeMsg) {
		if ("PropertyAdded" in msg) {
			if ("Channel" in msg.PropertyAdded.prop) {
				this.addChannel(Channel.fromJson(msg.PropertyAdded.prop.Channel));
			} else if ("Client" in msg.PropertyAdded.prop) {
				this.addClient(Client.fromJson(msg.PropertyAdded.prop.Client));
			} else if ("Server" in msg.PropertyAdded.prop) {
				this.updateServer(msg.PropertyAdded.prop.Server);
			}
		} else if ("PropertyChanged" in msg) {
			if ("Channel" in msg.PropertyChanged.prop && "Channel" in msg.PropertyChanged.id) {
				this.updateChannel(msg.PropertyChanged.id.Channel, msg.PropertyChanged.prop.Channel);
			} else if ("Client" in msg.PropertyChanged.prop && "Client" in msg.PropertyChanged.id) {
				this.updateClient(msg.PropertyChanged.id.Client, msg.PropertyChanged.prop.Client);
			} else if ("Server" in msg.PropertyChanged.prop) {
				this.updateServer(msg.PropertyChanged.prop.Server);
			}
		} else if ("PropertyRemoved" in msg) {
			if ("Channel" in msg.PropertyRemoved.id) {
				this.removeChannel(msg.PropertyRemoved.id.Channel);
			} else if ("Client" in msg.PropertyRemoved.id) {
				this.removeClient(msg.PropertyRemoved.id.Client);
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
	public icon_id!: number;
	public avatar_hash!: string;

	protected constructor() { }

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

	public getUid(): string | undefined {
		if (!this.uid)
			return;
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
	public client_type!: string;
	public country_code!: string;
	public database_id!: number;
	public description!: string
	public icon_id!: number;
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
	public uid!: number[];
	public unread_messages!: number;

	public volume?: number;
	/// true if whispering, false if talking, undefined if silent
	public talking?: boolean;

	// ITreeParent
	public children: Writable<ITreeNode[]> = writable([]);

	protected constructor() { super(); }

	public static fromJson(obj: any): Client {
		const c = new Client();
		Object.assign(c, obj);
		return c;
	}

	public update(obj: any): void {
		Object.assign(this, obj);
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
			client: this.getUid(),
			volume,
		});
	}

	public async loadVolume() {
		const res = await graphql(`query GetClientVolume($client: ID!) {
			client(uid: $client) { volume }
		}`, {
			client: this.getUid(),
		});
		if (res.data)
			this.volume = res.data.client.volume;
	}
}

export class Channel implements ITreeParent, ITreeNode {
	public id!: ChannelId;
	public parent!: ChannelId;
	public name!: string;
	public topic!: string | null;
	public codec!: string | null; // TODO enum
	public codec_quality!: number | null;
	public max_clients!: any;
	public max_family_clients!: any | null;
	public order!: ChannelId;
	public channel_type!: string; // TODO enum
	public is_default!: boolean | null;
	public has_password!: boolean | null;
	public codec_latency_factor!: number | null;
	public is_unencrypted!: boolean | null;
	public delete_delay!: any | null;
	public needed_talk_power!: number | null;
	public forced_silence!: boolean | null;
	public phonetic_name!: string | null;
	public icon_id!: any | null;
	public is_private!: boolean | null;
	public subscribed!: boolean;
	public permission_hints!: any | null;
	public optional_data!: any | null;

	// ITreeParent
	public children: Writable<ITreeNode[]> = writable([]);

	private constructor() { }

	public static fromDebug(id: ChannelId, parent: ChannelId, order: ChannelId): Channel {
		const c = new Channel();
		c.id = id;
		c.parent = parent;
		c.order = order;
		return c;
	}

	public static fromJson(obj: any): Channel {
		const c = new Channel();
		Object.assign(c, obj);
		return c;
	}

	public update(obj: any): void {
		Object.assign(this, obj);
	}

	// XXX Temporary
	public set_name(name: string): Channel {
		this.name = name;
		return this;
	}
}

export class Server implements ITreeParent {
	public name!: string;
	public phonetic_name!: string;
	public public_key?: number[];
	// Base64 encoded, result from graphql
	public publicKey?: string;

	// ITreeParent
	public children: Writable<ITreeNode[]> = writable([]);

	public update(obj: any): void {
		Object.assign(this, obj);
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

export interface ITreeParent {
	children: Writable<ITreeNode[]>;
}

export interface ITreeNode {
}
