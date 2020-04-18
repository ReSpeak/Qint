import { Writable, writable, Readable, derived, get } from "svelte/store";
import { InBookChangeMsg } from "../structs/ws";

type ChannelId = number;

export class Book {
	public server: Writable<Server> = writable(new Server());
	public clients: Writable<Map<number, Client>> = writable(new Map());
	public channels: Writable<Map<number, Channel>> = writable(new Map());

	public addChannel(channel: Channel) {
		this.channels.update(channels => {
			if (channels.has(channel.id)) throw Error(`Channel ${channel.id} already exists`);
			channels.set(channel.id, channel);
			let parent: ITreeParent | undefined;
			if (channel.parent === 0) parent = get(this.server);
			else parent = channels.get(channel.parent);
			if (parent) {
				parent.children.update(pch => [...pch, channel]); // TODO sorted
			}
			return channels;
		});
	}

	public updateChannel(id: number, obj: any) {
		const channel = this.getChannel(id);
		if (channel === undefined) {
			console.error(`Cannot update non-existant channel ${id}`);
			return;
		}
		channel.update(obj);
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

	public addClient(client: Client) {
		this.clients.update(clients => {
			if (clients.has(client.id)) throw Error(`Client ${client.id} already exists`);
			clients.set(client.id, client);
			let parent: ITreeParent = get(this.channels).get(client.channel);
			parent.children.update(pch => [client, ...pch]); // TODO sorted
			return clients;
		});
	}

	public updateClient(id: number, obj: any) {
		const client = this.getClient(id);
		if (client === undefined) {
			console.error(`Cannot update non-existant client ${id}`);
			return;
		}
		if ("channel" in obj) {
			let parent = this.getChannel(client.channel);
			if (parent !== undefined) {
				parent.children.update(pch => { pch.remove_item(client); return pch; });
			}
			parent = this.getChannel(obj.channel);
			if (parent !== undefined) {
				parent.children.update(pch => [client, ...pch]);
			}
		}
		client.update(obj);
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

	public getNode(id: number): Server | Channel | undefined {
		if (id === 0)
			return get(this.server) as Server;
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
			}
		} else if ("PropertyChanged" in msg) {
			if ("Channel" in msg.PropertyChanged.prop && "Channel" in msg.PropertyChanged.id) {
				this.updateChannel(msg.PropertyChanged.id.Channel, msg.PropertyChanged.prop.Channel);
			} else if ("Client" in msg.PropertyChanged.prop && "Client" in msg.PropertyChanged.id) {
				this.updateClient(msg.PropertyChanged.id.Client, msg.PropertyChanged.prop.Client);
			}
		} else if ("PropertyRemoved" in msg) {
			if ("Channel" in msg.PropertyRemoved.id) {
				this.removeChannel(msg.PropertyRemoved.id.Channel);
			} else if ("Client" in msg.PropertyRemoved.id) {
				this.removeClient(msg.PropertyRemoved.id.Client);
			}
		}
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
		c.name = obj.client.customName || obj.client.name;
		c.icon_id = obj.icon || 0;
		c.avatar_hash = obj.avatar || "";
		return c;
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
}

export class Client extends GraphQlClient implements ITreeNode {
	public avatar_hash!: string;
	public ​​​​away_message?: string;
	public ​​​​badges!: string;
	public ​​​​channel!: number;
	public ​​​​channel_group!: number;
	public ​​​​client_type!: string;
	public ​​​​country_code!: string;
	public ​​​​database_id!: number;
	public ​​​​description!: string
	public ​​​​icon_id!: number;
	public ​​​​id!: number;
	public ​​​​inherited_channel_group_from_channel!: number;
	public ​​​​input_hardware_enabled!: boolean;
	public ​​​​input_muted!: boolean;
	public ​​​​is_channel_commander!: boolean;
	public ​​​​is_priority_speaker!: boolean;
	public ​​​​is_recording!: boolean;
	public ​​​​metadata!: string
	public ​​​​name!: string;
	public ​​​​needed_serverquery_view_power!: number;
	public ​​​​output_hardware_enabled!: boolean;
	public ​​​​output_muted!: boolean;
	public ​​​​output_only_muted!: boolean;
	public ​​​​permission_hints?: string;
	public ​​​​phonetic_name!: string;
	public ​​​​talk_power!: number;
	public ​​​​talk_power_granted!: boolean;
	public ​​​​talk_power_request?: string;
	public ​​​​uid!: number[];
	public unread_messages!: number;

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
}

export class Channel implements ITreeParent, ITreeNode {
	public id!: ChannelId;
	public name?: string;
	public parent!: ChannelId;
	public order!: ChannelId;

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
	public name?: string;

	// ITreeParent
	public children: Writable<ITreeNode[]> = writable([]);
}

export interface ITreeParent {
	children: Writable<ITreeNode[]>;
}

export interface ITreeNode {
}
