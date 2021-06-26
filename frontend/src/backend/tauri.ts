import { InMsg, OutMsg } from "./ws";
import { BASE_ADDRESS, createUuidV4 } from "../util";
import { closedFn, errorFn, IBackend, IBackendConnection, IFetchLike, msgFn } from "./backend";
import { emit, listen } from "@tauri-apps/api/event";
import { urlToWebSocket } from "./backendUtil";
import debug from "debug";
import { invoke } from "@tauri-apps/api/tauri";
const log = debug("TAURI");

type TauriMsg<T> = { Msg: T } | "Close";
type TauriWs<T> = {
	connection: string,
	msg: TauriMsg<T>,
}
type TauriMsgP2F = TauriWs<InMsg>;
type TauriMsgF2P = TauriWs<OutMsg>;


type OutHttpRequest = OutListPluginsRequest;

interface OutListPluginsRequest {
	ListPlugins: unknown;
}

class FetchLike implements IFetchLike {
	constructor(private obj: any) { }
	public async json(): Promise<any> {
		return this.obj;
	}
	public async text(): Promise<string> {
		return JSON.stringify(this.obj);
	}
}

export class TauriBackend implements IBackend {
	public cacheFileSrc: string;
	public readonly wsBaseAddress: string = urlToWebSocket(BASE_ADDRESS);

	private connections: Map<string, TauriBackendConnection> = new Map();

	constructor() {
		log("Using tauri backend");
		this.cacheFileSrc = `${BASE_ADDRESS}/filecache`;

		listen<string>("ws", (ev) => {
			log("Ws: %o", ev);
			const msg = JSON.parse(ev.payload) as TauriMsgP2F;
			const con = this.connections.get(msg.connection);
			if (con !== undefined) {
				if (msg.msg === "Close") {
					con.onClose?.();
					this.connections.delete(msg.connection);
				}
				else con.onMsg?.(msg.msg.Msg);
			}
		});
	}

	createNewConnection(): IBackendConnection {
		const con = new TauriBackendConnection();
		this.connections.set(con.id, con);
		return con;
	}

	private async listPlugins(): Promise<string[]> {
		//return (await promisified<{ PluginList: string[] }>({ ListPlugins: {} })).PluginList;
		return []; // ?TAURI
	}

	private async getPlugin(name: string): Promise<string> {
		//return (await promisified<{ Plugin: string }>({ GetPlugin: name })).Plugin;
		return ""; // ?TAURI
	}

	public async fetch(cmd: string, data: RequestInit): Promise<IFetchLike> {
		if (cmd === "/plugins") {
			return new FetchLike(this.listPlugins());
		} else if (cmd.startsWith("/plugins/")) {
			return new FetchLike(this.getPlugin(cmd.slice("/plugins/".length)));
		}
		return fetch(`${BASE_ADDRESS}${cmd}`, data);
	}

	public async graphql<T = any>(
		query: string,
		variables?: Record<string, unknown>
	): Promise<{ data: T }> {
		//return (await promisified<{ Graphql: any }>({ Graphql: { query, variables } })).Graphql;
		return { data: undefined as any }; // ?TAURI
	}

	public setTitle(name: string): void {
		document.title = name;
	}

	public setIcon(url: string | undefined): void {
		const icon = document.querySelector("link[rel*='icon']") as HTMLLinkElement;
		if (icon !== null) icon.href = url ?? "icon.png";
		else console.log("Tried to set icon but did not find icon element");
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

	public async connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void> {
		this.onMsg = onMsg;
		this.onClose = onClose;

		invoke("create_ws", { uuid: this.id });
		log("create_ws %j", this.id);

		this.serverFileSrc = `${BASE_ADDRESS}/con/${this.id}`;
	}

	public send(data: OutMsg): void {
		const i: TauriMsgF2P = {
			connection: this.id,
			msg: { Msg: data },
		};
		invoke("pass_ws_msg", i);
		log("pass_ws_msg %j", i);
	}

	public close(): void {
		const i: TauriMsgF2P = {
			connection: this.id,
			msg: "Close",
		};
		invoke("pass_ws_msg", i);
		log("close %j", i);
		this.id = createUuidV4();
	}

	public async fetch(cmd: string, data: RequestInit): Promise<IFetchLike> {
		return fetch(`${this.serverFileSrc}${cmd}`, data);
	}
}
