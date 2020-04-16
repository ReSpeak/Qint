import { Chat, Message } from "./chat/chat";
import { OutMsg, InMsg } from "./structs/ws";
import { get, writable, Writable } from "svelte/store";
import { Book, Channel, Server } from "./tree/book";

export class Connection {
	public readonly state = writable(ConnectionState.Disconnected);
	public readonly error: Writable<string | undefined> = writable(undefined);

	public readonly book: Book = new Book();
	public readonly chat: Chat = new Chat(this);
	public server: string | undefined;
	private socket: WebSocket | undefined;
	private guid: string | undefined;

	constructor() {
		this.fillDummyData();
	}

	public reset() {
		this.state.set(ConnectionState.Disconnected);
		this.socket?.close();
		this.socket = undefined;
	}

	public connect(opt: IConnectOptions) {
		this.error.set(undefined);
		this.guid = "36c07459-a731-4868-9f10-a9b7564a4461"; // TODO random
		this.socket = new WebSocket(`ws://localhost:4422/con/${this.guid}/ws?format=Json`);
		this.socket.onopen = () => {
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
		};
		this.socket.onerror = (error) => {
			this.error.set("Connection failed, is Qint running?");
		};
		this.socket.onclose = () => this.reset();
		this.socket.onmessage = (evt) => this.messageHandler(evt);
		this.state.set(ConnectionState.Connecting);
	}

	private fillDummyData() {
		this.book.addChannel(Channel.fromDebug(1, 0, 0).set_name("A"));
		this.book.addChannel(Channel.fromDebug(2, 1, 0).set_name("B"));
		this.book.addChannel(Channel.fromDebug(3, 1, 2).set_name("C"));
		this.book.addChannel(Channel.fromDebug(4, 1, 2).set_name("C"));
		this.book.addChannel(Channel.fromDebug(5, 1, 2).set_name("C"));
		this.book.addChannel(Channel.fromDebug(6, 1, 2).set_name("C"));
		this.book.addChannel(Channel.fromDebug(7, 1, 2).set_name("C"));
		this.book.addChannel(Channel.fromDebug(8, 1, 2).set_name("C"));
		this.book.addChannel(Channel.fromDebug(9, 1, 2).set_name("C"));
		this.book.server.update(s => { s.name = "Server der Verplanten"; return s; });
		this.chat.messages.update(m => [...m,
			new Message("0", "asd", "asdfg"),
			new Message("0", "asd", "asdfg"),
			new Message("0", "foor", "asdfg"),
			new Message("0", "as<>d", "a<div>sdfg"),
			new Message("0", "asd", "asdfg\nasdgf"),
			new Message("0", "asd", "asdfg\nasdgf\nasdgf\nasdgf\nasdgf"),
			new Message("0", "asd", "asdfg\nasdgf\nasdgf\nasdgf\nasdgf"),
			new Message("0", "asd", "asdfg\nasdgf\nasdgf\nasdgf\nasdgf"),
			new Message("0", "asd", "asdfg\nasdgf\nasdgf\nasdgf\nasdgf"),
			new Message("0", "asd", "asdfg\nasdgf\nasdgf\nasdgf\nasdgf"),
			new Message("0", "asd", "asdfg\nasdgf\nasdgf\nasdgf\nasdgf"),
			new Message("0", "asd", "asdfg\nasdgf\nasdgf\nasdgf\nasdgf"),
			new Message("0", "asd", "asdfg\nasdgf\nasdgf\nasdgf\nasdgf"),
			new Message("0", "asd", "asdfg\nasdgf\nasdgf\nasdgf\nasdgf"),
			new Message("0", "asd", "asdfg\nasdgf\nasdgf\nasdgf\nasdgf"),
		]);
	}

	public sendMessage(data: OutMsg): void {
		this.socket?.send(JSON.stringify(data));
	}

	public sendRawMessage(data: string): void {
		this.socket?.send(data);
	}

	private messageHandler(evt: MessageEvent) {
		const msg = JSON.parse(evt.data) as InMsg;
		if ("Connected" in msg) {
			this.state.set(ConnectionState.Connected);
			this.server = msg.Connected.server;
		} else if ("Events" in msg) {
			for (const tsevt of msg.Events) {
				console.log(tsevt);
			}
		} else if ("TalkersChanged" in msg) {
			// TODO
		} else if ("Error" in msg) {
			console.warn("Con Error:", msg.Error);
			if (get(this.state) == ConnectionState.Connecting) {
				this.socket?.close();
				this.error.set(msg.Error);
			}
		} else {
			console.error("Unknown message", msg);
		}
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
