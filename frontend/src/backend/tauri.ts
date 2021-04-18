import { InMsg, OutMsg } from "./ws";
import { BASE_ADDRESS, createUuidV4 } from "../util";
import { closedFn, errorFn, IBackend, IBackendConnection, IFetchLike, msgFn } from "./backend";
import { emit, listen } from 'tauri/api/event';
import { promisified } from 'tauri/api/tauri'
import { urlToWebSocket } from "./backendUtil";

type OutWsEvent = { Msg: OutMsg } | "Close";

interface OutWsMsg {
	connection: string;
	msg: OutWsEvent;
}

type InWsEvent = { Msg: InMsg } | "Close";

interface InWsMsg {
	connection: string;
	msg: InWsEvent;
}

type OutHttpRequest = OutListPluginsRequest;

interface OutListPluginsRequest {
	ListPlugins: unknown;
}

const connections: Map<string, TauriBackendConnection> = new Map();

listen<string>('websocket', payload => {
	const msg = JSON.parse(payload.payload) as InWsMsg;
	const con = connections.get(msg.connection);
	if (msg.msg === "Close")
		con?.onClose?.();
	else
		con?.onMsg?.(msg.msg.Msg);
});

class FetchLike implements IFetchLike {
	constructor(private obj: any) {}
	public async json(): Promise<any> { return this.obj; }
	public async text(): Promise<string> { return JSON.stringify(this.obj); }
}

export class TauriBackend implements IBackend {
	public cacheFileSrc: string;
	public readonly wsBaseAddress: string = urlToWebSocket(BASE_ADDRESS);

	constructor() {
		this.cacheFileSrc = `${BASE_ADDRESS}/filecache`;
	}

	createNewConnection(): IBackendConnection {
		return new TauriBackendConnection();
	}

	private async listPlugins(): Promise<string[]> {
		return (await promisified<{ PluginList: string[] }>({ ListPlugins: {} })).PluginList;
	}

	private async getPlugin(name: string): Promise<string> {
		return (await promisified<{ Plugin: string }>({ GetPlugin: name })).Plugin;
	}

	public async fetch(cmd: string, data: RequestInit): Promise<IFetchLike> {
		if (cmd === "/plugins") {
			return new FetchLike(this.listPlugins());
		} else if (cmd.startsWith("/plugins/")) {
			return new FetchLike(this.getPlugin(cmd.slice("/plugins/".length)));
		}
		return fetch(`${BASE_ADDRESS}${cmd}`, data);
	}

	public async graphql<T = any>(query: string, variables?: Record<string, unknown>): Promise<{ data: T }> {
		return (await promisified<{ Graphql: any }>({ Graphql: { query, variables } })).Graphql;
	}

	public setTitle(name: string): void {
		document.title = name;
	}

	public setIcon(url: string | undefined): void {
		const icon = document.querySelector("link[rel*='icon']") as HTMLLinkElement;
		if (icon !== null)
			icon.href = url ?? "icon.png";
		else
			console.log("Tried to set icon but did not find icon element");
	}
}

export class TauriBackendConnection implements IBackendConnection {
	public serverFileSrc: string;
	public id: string;
	onMsg?: msgFn;
	onClose?: closedFn;

	constructor() {
		this.serverFileSrc = "";
		this.id = createUuidV4();
	}

	public send(data: OutMsg): void {
		const msg: OutWsMsg = {
			connection: this.id,
			msg: { Msg: data },
		};
		emit("websocket", JSON.stringify(msg));
	}

	public async connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void> {
		this.close();
		this.onMsg = onMsg;
		this.onClose = onClose;

		this.serverFileSrc = `${BASE_ADDRESS}/con/${this.id}`;

		connections.set(this.id, this);
	}

	public close(): void {
		connections.delete(this.id);
		const msg: OutWsMsg = {
			connection: this.id,
			msg: "Close",
		};
		emit("websocket", JSON.stringify(msg));
		this.id = createUuidV4();
	}

	public async fetch(cmd: string, data: RequestInit): Promise<IFetchLike> {
		return fetch(`${this.serverFileSrc}${cmd}`, data);
	}
}
