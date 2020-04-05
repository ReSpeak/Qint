import { Writable, writable, Readable, derived, get } from "svelte/store";
import { InMsg } from "../structs/ws";

type ChannelId = number;

export class Book {
	public server: Writable<Server> = writable(new Server());
	public clients: Writable<Map<number, Client>> = writable(new Map());
	public channels: Writable<Map<number, Channel>> = writable(new Map());

	public addChannel(channel: Channel) {
		this.channels.update(channels => {
			if (channels.has(channel.id)) throw Error("Channel already exists");
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

	public getNode(id: number): Server | Channel | undefined {
		if (id === 0)
			return get(this.server) as Server;
		else
			return get(this.channels).get(id) as Channel | undefined;
	}

	public getChannel(id: number): Channel | undefined {
		if (id === 0)
			return undefined;
		else
			return get(this.channels).get(id) as Channel | undefined;
	}

	public removeChannel(id: number): void {
		if (id === 0) throw Error("Cannot remove Server (Id:0)");
		const channel = this.getChannel(id);
		if (channel === undefined) return;
		const parent = this.getNode(channel.parent);
		if (parent !== undefined) {
			parent.children.update(c => { c.remove_item(channel.id); return c; });
		}
	}

	public handleBookMessage(msg: InMsg) {
		const enu = Object.keys(msg)[0];
		switch (enu) {
			case "b_add":
				this.handleAdd(msg);
				break;
			case "b_change":
				this.handleChange(msg);
				break;
			case "b_remove":
		}
	}

	private handleAdd(msg: any): void {
		if (msg.to === "channel") {
			this.addChannel(Channel.fromJson(msg.obj));
		}
	}

	private handleChange(msg: any): void {
		if (msg.to === "channel") {
			if (msg.obj.id === undefined) throw Error("Missing object id");
			const channel = this.getChannel(msg.obj.id);
			if (channel === undefined) throw Error("Channel not found");
			const old_parent = channel?.parent;
			const old_order = channel?.parent;
			channel.update(msg.obj);
			if (channel.parent !== old_parent || channel.order !== old_order) {
				this.removeChannel(channel.id);
				this.addChannel(channel);
			}
		}
	}
}

export class Client implements ITreeNode {
	public name?: string;
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
