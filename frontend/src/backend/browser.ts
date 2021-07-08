import { InMsg, OutMsg } from "./ws";
import { BASE_ADDRESS, createUuidV4 } from "../util";
import { closedFn, errorFn, IBackend, IBackendConnection, ICacheFileRequest, IFetchLike, IFileRequest, msgFn } from "./backend";
import { urlToWebSocket } from "./backendUtil";

export class BrowserBackend implements IBackend {
	public name = "Browser";
	public readonly cacheFileSrc: string;
	public readonly wsBaseAddress: string = urlToWebSocket(BASE_ADDRESS);

	constructor() {
		this.cacheFileSrc = `${BASE_ADDRESS}/filecache`;
	}

	createNewConnection(): IBackendConnection {
		return new BrowserBackendConnection(this);
	}

	public fetch(cmd: string, data?: RequestInit): Promise<IFetchLike> {
		return fetch(`${BASE_ADDRESS}${cmd}`, data);
	}

	public async graphql<T = any>(
		query: string,
		variables?: Record<string, unknown>
	): Promise<{ data: T }> {
		const val = await this.fetch(`/db`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ query, variables }),
		});
		return await val.json();
	}

	public setTitle(name: string): void {
		document.title = name;
	}

	public setIcon(url: string | undefined): void {
		const icon = document.querySelector("link[rel*='icon']") as HTMLLinkElement;
		if (icon !== null) icon.href = url ?? "icon.png";
		else console.log("Tried to set icon but did not find icon element");
	}

	public async get_settings(): Promise<Record<string, unknown>> {
		const resp = await this.fetch("/settings");
		return await resp.json();
	}

	public async set_settings(diff: Record<string, unknown>): Promise<void> {
		await this.fetch(`/settings`, {
			method: "PUT",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify(diff),
		});
	}

	public fetch_cache_image(req: ICacheFileRequest): Promise<string> {
		let str = `${this.cacheFileSrc}/${req.server}/${req.channel}${req.path}`;
		let hasQ = false;
		if (req.hash) { str += (hasQ ? "&" : "?") + "hash=" + encodeURIComponent(req.hash); hasQ = true; }
		return Promise.resolve(str);
	}
}

export class BrowserBackendConnection implements IBackendConnection {
	public serverFileSrc: string;
	public id: string;
	private socket?: WebSocket;

	constructor(private parent: BrowserBackend) {
		this.serverFileSrc = "";
		this.id = createUuidV4();
	}

	public send(data: OutMsg): void {
		if (this.socket) this.socket.send(JSON.stringify(data));
	}

	public connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void> {
		this.close();

		this.serverFileSrc = `${BASE_ADDRESS}/con/${this.id}`;
		this.socket = new WebSocket(`${this.parent.wsBaseAddress}/con/${this.id}/ws`);
		this.socket.onerror = (error) => onError(String(error));
		this.socket.onclose = onClose;
		this.socket.onmessage = (evt) => {
			onMsg(JSON.parse(evt.data) as InMsg);
		};
		return new Promise((resolve) => {
			this.socket!.onopen = () => {
				resolve();
			};
		});
	}

	public close(): void {
		if (this.socket) this.socket.close();
		this.socket = undefined;
	}

	public fetch(cmd: string, data: RequestInit): Promise<IFetchLike> {
		return fetch(`${this.serverFileSrc}${cmd}`, data);
	}

	public fetch_image(req: IFileRequest): Promise<string> {
		let str = `${this.serverFileSrc}/file/${req.channel}${req.path}`;
		let hasQ = false;
		if (req.cache) { str += (hasQ ? "&" : "?") + "cache=true"; hasQ = true; }
		if (req.hash) { str += (hasQ ? "&" : "?") + "hash=" + encodeURIComponent(req.hash); hasQ = true; }
		return Promise.resolve(str);
	}
}
