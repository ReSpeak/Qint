import { InMsg, OutMsg } from "./ws";
import { BASE_ADDRESS } from "../util";
import { closedFn, errorFn, IBackend, IBackendConnction, IFetchLike, msgFn } from "./backend";

export class BrowserBackend implements IBackend {
	public cacheFileSrc: string;

	constructor() {
		this.cacheFileSrc = `${BASE_ADDRESS}/filecache`;
	}

	createNewConnection(): IBackendConnction {
		return new BrowserBackendConnection();
	}

	public fetch(cmd: string, data: RequestInit): Promise<IFetchLike> {
		return fetch(`${BASE_ADDRESS}${cmd}`, data);
	}

	public setTitle(name: string): void {
		document.title = name;
	}
}

export class BrowserBackendConnection implements IBackendConnction {
	public serverFileSrc: string;
	private guid?: string;
	private socket?: WebSocket;

	constructor() {
		this.serverFileSrc = "";
	}
	getGuidTmpHack(): string {
		return this.guid!;
	}

	public send(data: OutMsg): void {
		if (this.socket)
			this.socket.send(JSON.stringify(data));
	}
	public connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void> {
		close();

		this.guid = BrowserBackendConnection.createUuidV4();
		this.serverFileSrc = `${BASE_ADDRESS}/con/${this.guid}`;

		let path = BASE_ADDRESS;
		if (!path.startsWith("http"))
			path = window.location.origin;
		if (!path.startsWith("http"))
			throw Error("Failed to get websocket path");
		// Replace http by ws, so https gets wss
		path = path.slice(4);

		this.socket = new WebSocket(`ws${path}/con/${this.guid}/ws?format=Json`);
		this.socket.onerror = (error) => onError(String(error));
		this.socket.onclose = onClose;
		this.socket.onmessage = (evt) => { onMsg(JSON.parse(evt.data) as InMsg); };
		return new Promise((resolve, reject) => {
			this.socket!.onopen = () => { resolve() };
		});
	}
	public close(): void {
		if (this.socket)
			this.socket.close();
		this.guid = undefined;
		this.socket = undefined;
	}

	// See https://jsperf.com/node-uuid-performance/64 about how to generate a uuid fast
	private static createUuidV4(): string {
		var d2h: string[] = [], vals = new Array(16);
		for (var i = 0; i < 256; ++i) d2h.push((0x100 + i).toString(16).substr(1));

		for (var i = 0; i < 16; ++i) vals[i] = Math.random() * 256 | 0;
		vals[6] = vals[6] & 0x0f | 0x40;
		vals[8] = vals[8] & 0x3f | 0x80;
		return d2h[vals[0]] + d2h[vals[1]] + d2h[vals[2]] + d2h[vals[3]] +
			'-' + d2h[vals[4]] + d2h[vals[5]] +
			'-' + d2h[vals[6]] + d2h[vals[7]] +
			'-' + d2h[vals[8]] + d2h[vals[9]] +
			'-' + d2h[vals[10]] + d2h[vals[11]] + d2h[vals[12]] + d2h[vals[13]] + d2h[vals[14]] + d2h[vals[15]];
	}
}
