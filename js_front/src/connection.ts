import { Chat, Message } from "./chat/chat";
import { OutMsg, InMsg } from "./structs/ws";
import { writable, Writable } from "svelte/store";
import { Book, Channel, Server } from "./tree/book";

export class Connection {
	public readonly state = writable(ConnectionState.Disconnected);

	public readonly book: Book;
	public readonly chat: Chat;
	private socket: WebSocket;
	private guid: string;

	constructor() {
		this.book = new Book();
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
		this.book.addChannel(new Channel(1, 0, 0).set_name("A"));
		this.book.addChannel(new Channel(2, 1, 0).set_name("B"));
		this.book.addChannel(new Channel(3, 1, 2).set_name("C"));
		this.book.server.update(s => { s.name = "Server der Verplanten"; return s; });
		this.chat.messages.update(m => [...m,
		new Message("asd", "asdfg"),
		new Message("asd", "asdfg"),
		new Message("foor", "asdfg"),
		new Message("as<>d", "a<div>sdfg"),
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
