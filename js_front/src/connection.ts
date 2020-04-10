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
		this.chat = new Chat(this);
		this.guid = "36c07459-a731-4868-9f10-a9b7564a4461"; // TODO random
		this.socket = new WebSocket(`ws://localhost:4422/con/${this.guid}/ws?format=Json`);
		//this.socket = new WebSocket("ws://localhost:2319");
		this.socket.onmessage = (evt) => this.messageHandler(evt);
		//this.socket.send("{\"test\":\"sdf\"}");

		this.fillDummyData();
	}

	public connect(opt: IConnectOptions) {
		this.sendMessage({
			Connect: {
				address: opt.address,
				name: opt.name,
				log_commands: false,
				log_packets: false,
				log_udp_packets: false,
				version: "Linux_5_0_0_test_87"
			}
		});
		this.state.set(ConnectionState.Connecting);
	}

	private fillDummyData() {
		//this.state.set(ConnectionState.Connected);
		this.book.addChannel(Channel.fromDebug(1, 0, 0).set_name("A"));
		this.book.addChannel(Channel.fromDebug(2, 1, 0).set_name("B"));
		this.book.addChannel(Channel.fromDebug(3, 1, 2).set_name("C"));
		this.book.server.update(s => { s.name = "Server der Verplanten"; return s; });
		this.chat.messages.update(m => [...m,
		new Message("asd", "asdfg"),
		new Message("asd", "asdfg"),
		new Message("foor", "asdfg"),
		new Message("as<>d", "a<div>sdfg"),
		new Message("asd", "asdfg"),
		]);
	}

	public sendMessage(data: OutMsg): void {
		this.socket.send(JSON.stringify(data));
	}

	public sendRawMessage(data: string): void {
		this.socket.send(data);
	}

	private messageHandler(evt: MessageEvent) {
		const msg = JSON.parse(evt.data) as InMsg;
		if ("Events" in msg) {
			for (const tsevt of msg.Events) {
				console.log(tsevt);
			}
		} else if ("TalkersChanged" in msg) {
			// TODO
		} else if ("Error" in msg) {
			console.warn("Con Error:", msg.Error);
		} else {
			console.error("Unknown message", msg);
		}
		this.state.set(ConnectionState.Connected);

	}

	private takenumber(a: number) {}
}

export enum ConnectionState {
	Disconnected,
	Connecting,
	Connected,
}

interface IConnectOptions {
	address: string;
	name: string;
	// ...
}
