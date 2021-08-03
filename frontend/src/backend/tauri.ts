import { InMsg, OutMsg } from "./ws";
import { BASE_ADDRESS, createUuidV4 } from "../util";
import {
	closedFn,
	errorFn,
	IAudioDeviceList,
	IBackend,
	IBackendConnection,
	ICacheFileRequest,
	IFetchLike,
	IFileRequest,
	msgFn,
} from "./backend";
import { listen } from "@tauri-apps/api/event";
import { urlToWebSocket } from "./backendUtil";
import debug from "debug";
import { invoke } from "@tauri-apps/api/tauri";
import { RustAnalyzeResult } from "src/chat/previewAnalyzer";
const log = debug("TAURI");

type TauriMsg<T> = { Msg: T } | "Close";
type TauriWs<T> = {
	connection: string;
	msg: TauriMsg<T>;
};
type TauriMsgP2F = TauriWs<InMsg>;
type TauriMsgF2P = TauriWs<OutMsg>;

type OutHttpRequest = OutListPluginsRequest;

interface OutListPluginsRequest {
	ListPlugins: unknown;
}

class FetchLike implements IFetchLike {
	constructor(private obj: any) {}
	public async json(): Promise<any> {
		return this.obj;
	}
	public async text(): Promise<string> {
		return JSON.stringify(this.obj);
	}
}

class ImageTracking {
	private trackedImages: Map<string, string | Promise<string>> = new Map();

	protected async fetchImgInternal(ep: string, req: IFileRequest, con: string) {
		const key = reqAsKey(req);
		let url = this.trackedImages.get(key);
		if (url === undefined) {
			const task = (async () => {
				const response = await invoke<GetFileResponse>(ep, {
					req: {
						con,
						channel: req.channel,
						path: req.path,
						hash: req.hash,
						cache: req.cache,
					},
				});
				const buffer = new Uint8Array(response.data);
				const blob = new Blob([buffer], {
					type: response.mime,
				});
				url = URL.createObjectURL(blob);
				return url;
			})();
			this.trackedImages.set(key, task);
			url = await task;
			this.trackedImages.set(key, url);
			return url;
		} else {
			return await url;
		}
	}

	protected releaseImages() {
		this.trackedImages.forEach(async (v) => URL.revokeObjectURL(await v));
		this.trackedImages.clear();
	}
}

export class TauriBackend extends ImageTracking implements IBackend {
	public name = "Tauri";
	public cacheFileSrc: string;
	public readonly wsBaseAddress: string = urlToWebSocket(BASE_ADDRESS);

	private connections: Map<string, TauriBackendConnection> = new Map();

	constructor() {
		super();
		log("Using tauri backend");
		this.cacheFileSrc = `${BASE_ADDRESS}/filecache`;

		listen<TauriMsgP2F>("ws", (ev) => {
			log("QintConnection: %o", ev);
			const msg = ev.payload;
			const con = this.connections.get(msg.connection);
			if (con !== undefined) {
				if (msg.msg === "Close") {
					con.onClose?.();
					this.connections.delete(msg.connection);
				} else con.onMsg?.(msg.msg.Msg);
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
		const resp = await invoke<string>("db", { request: { query, variables } });
		return JSON.parse(resp);
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
		return await invoke<Record<string, unknown>>("get_settings");
	}

	public async set_settings(diff: Record<string, unknown>): Promise<void> {
		await invoke("set_settings", { diff });
	}

	public async fetch_cache_image(req: ICacheFileRequest): Promise<string> {
		return await this.fetchImgInternal("get_cache_file", req, req.server);
	}
	
	public async peek_link(link: string): Promise<RustAnalyzeResult> {
		return await invoke<RustAnalyzeResult>("peek_link", { link });
	}

	public async get_audio_device_list() : Promise<IAudioDeviceList> {
		return await invoke<IAudioDeviceList>("get_audio_device_list");
	}
}

export class TauriBackendConnection extends ImageTracking implements IBackendConnection {
	public serverFileSrc: string;
	public id: string;
	onMsg?: msgFn;
	onClose?: closedFn;

	constructor() {
		super();
		this.serverFileSrc = "";
		this.id = createUuidV4();
	}

	public async connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void> {
		this.onMsg = onMsg;
		this.onClose = onClose;

		await invoke("create_ws", { uuid: this.id });
		log("create_ws %j", this.id);

		this.serverFileSrc = `${BASE_ADDRESS}/con/${this.id}`;
	}

	public async send(data: OutMsg): Promise<void> {
		const i: TauriMsgF2P = {
			connection: this.id,
			msg: { Msg: data },
		};
		await invoke("pass_ws_msg", i);
		log("pass_ws_msg %j", i);
	}

	public async close(): Promise<void> {
		const i: TauriMsgF2P = {
			connection: this.id,
			msg: "Close",
		};
		log("closing %j", i);
		this.releaseImages();
		await invoke("pass_ws_msg", i);
		this.id = createUuidV4();
		log("closed %j", i);
	}

	public async fetch(cmd: string, data: RequestInit): Promise<IFetchLike> {
		return fetch(`${this.serverFileSrc}${cmd}`, data);
	}

	public async fetch_image(req: IFileRequest): Promise<string> {
		return await this.fetchImgInternal("get_file", req, this.id);
	}
}

function reqAsKey(req: IFileRequest | ICacheFileRequest): string {
	return `${req.channel}/${req.path}`;
}
interface GetFileResponse {
	data: ArrayLike<number>;
	mime: string | undefined;
}
