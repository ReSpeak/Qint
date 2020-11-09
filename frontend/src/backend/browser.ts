import { InMsg, OutMsg } from "./ws";
import { BASE_ADDRESS, createUuidV4 } from "../util";
import { closedFn, errorFn, IBackend, IBackendConnection, IFetchLike, msgFn } from "./backend";
import { urlToWebSocket } from "./backendUtil";

export class BrowserBackend implements IBackend {
	public readonly cacheFileSrc: string;
	public readonly wsBaseAddress: string = urlToWebSocket(BASE_ADDRESS);

	constructor() {
		this.cacheFileSrc = `${BASE_ADDRESS}/filecache`;
	}

	createNewConnection(): IBackendConnection {
		return new BrowserBackendConnection(this);
	}

	public fetch(cmd: string, data: RequestInit): Promise<IFetchLike> {
		return fetch(`${BASE_ADDRESS}${cmd}`, data);
	}

	public async graphql<T = any>(query: string, variables?: object): Promise<{ data: T }> {
		const val = await this.fetch(`/db`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ query, variables })
		});
		return await val.json();
	}

	public setTitle(name: string): void {
		document.title = name;
	}
}

export class BrowserBackendConnection implements IBackendConnection {
	public serverFileSrc: string;
	public id: string;
	private socket?: WebSocket;

	constructor(
		private parent: BrowserBackend
	) {
		this.serverFileSrc = "";
		this.id = createUuidV4();
	}

	public send(data: OutMsg): void {
		if (this.socket)
			this.socket.send(JSON.stringify(data));
	}

	public connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void> {
		this.close();

		this.serverFileSrc = `${BASE_ADDRESS}/con/${this.id}`;

		this.socket = new WebSocket(`${this.parent.wsBaseAddress}/con/${this.id}/ws?format=Json`);
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
		this.id = createUuidV4();
		this.socket = undefined;
	}

	public fetch(cmd: string, data: RequestInit): Promise<IFetchLike> {
		return fetch(`${this.serverFileSrc}${cmd}`, data);
	}
}
