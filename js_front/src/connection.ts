import { Chat, Message } from "./chat/chat";
import { OutMsg, InMsg } from "./msg/ws";
import { writable } from "svelte/store";

export class Connection {
	public state = writable(ConnectionState.Disconnected);
	public server?: Server;
	public clients: Map<number, Client> = new Map();
	public channels: Map<number, Channel> = new Map();
	public chat: Chat;
	private socket: WebSocket;
	private guid: string;

	constructor() {
		this.chat = new Chat();
		this.guid = "36c07459-a731-4868-9f10-a9b7564a4461"; // TODO random
		// this.socket = new WebSocket(`ws://con/${this.guid}/ws`);
		this.socket = new WebSocket("ws://localhost:2319");
		this.socket.onmessage = (evt) => this.messageHandler(evt);

		this.fillDummyData();
	}

	public connect(opt: IConnectOptions) {
		this.sendMessage({
			_cmd: "connect",
			address: opt.address
		});
		this.state.set(ConnectionState.Connecting);
	}

	private fillDummyData() {
		this.state.set(ConnectionState.Connected);
		this.channels.set(3, new Channel(3, 0, 0));
		this.chat.messages.update(m => [...m,
			new Message("asd", "asdfg"),
			new Message("asd", "asdfg"),
			new Message("foor", "asdfg"),
			new Message("asd", "asdfg"),
			new Message("asd", "asdfg"),
		]);
	}

	private sendMessage(data: OutMsg): void {
		this.socket.send(JSON.stringify(data));
	}

	private messageHandler(evt: MessageEvent) {
		const msg = JSON.parse(evt.data) as InMsg;
		switch (msg._cmd) {
			case "b_add":
				// TODO
				break;
			case "b_change":
				break;
			case "b_remove":
				break;
			case "connected":
				this.state.set(ConnectionState.Connected);
				console.log("connected");
				break;
			default:
				console.warn("unknown packet", msg);
				break;
		}
	}

	// structure:
	// { _cmd:"b_add", to:"client", id:"42", obj: { name: "lullinger" } }
}

export enum ConnectionState {
	Disconnected,
	Connecting,
	Connected,
}

interface IConnectOptions {
	address: string;
	// ...
}

type ChannelId = number;

class Client {
	public name?: string;
}

class Channel {
	public id: ChannelId;
	public name?: string;
	public parent: ChannelId;
	public order: ChannelId;

	constructor(id: ChannelId, parent: ChannelId, order: ChannelId) {
		this.id = id;
		this.parent = parent;
		this.order = order;
	}
}

class Server {
	public name?: string;
}
