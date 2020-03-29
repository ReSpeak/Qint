import { Writable, writable, Readable, derived, get } from "svelte/store";

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
}

export class Client implements ITreeNode {
	public name?: string;
}

export class Channel implements ITreeParent, ITreeNode {
	public id: ChannelId;
	public name?: string;
	public parent: ChannelId;
	public order: ChannelId;

	// ITreeParent
	public children: Writable<ITreeNode[]> = writable([]);

	constructor(id: ChannelId, parent: ChannelId, order: ChannelId) {
		this.id = id;
		this.parent = parent;
		this.order = order;
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
