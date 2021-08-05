import { InMsg, OutMsg } from "./ws";
import { createUuidV4 } from "../util";
import {
	closedFn,
	errorFn,
	FindIdentity,
	IAudioDeviceList,
	IBackend,
	IBackendConnection,
	ICacheFileRequest,
	IFetchLike,
	IFileRequest,
	msgFn,
	UpdateIdentityOptions,
} from "./backend";
import { listen } from "@tauri-apps/api/event";
import debug from "debug";
import { invoke } from "@tauri-apps/api/tauri";
import { RustAnalyzeResult } from "../chat/previewAnalyzer";
import { ApiIdentity } from "../panel/settings/identity";
import { MuteStates } from "../connect/uiConnect";
import { HotkeyAction } from "../transientSettings";
import { importFunc, IPlugin } from "../plugins";
const log = debug("TAURI");

type TauriP2FWs = { con: string; msg: InMsg; };
type TauriP2FClose = string;
type TauriF2PCreate = { con: string; };
type TauriF2PWs = { con: string; msg: OutMsg; };
type TauriF2PClose = { con: string; };


type OutHttpRequest = OutListPluginsRequest;

interface OutListPluginsRequest {
	ListPlugins: unknown;
}


class ImageTracking {
	private trackedImages: Map<string, string | undefined | Promise<string | undefined>> = new Map();

	protected async fetchImgInternal(ep: string, req: IFileRequest, con: string): Promise<string | undefined> {
		const key = reqAsKey(req);
		if (this.trackedImages.has(key)) {
			return await this.trackedImages.get(key);
		} else {
			const task = this.fetchImgTask(ep, req, con);
			this.trackedImages.set(key, task);
			const resolvedUrl = await task;
			this.trackedImages.set(key, resolvedUrl);
			return resolvedUrl;
		}
	}

	private async fetchImgTask(ep: string, req: IFileRequest, con: string): Promise<string | undefined> {
		try {
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
			return URL.createObjectURL(blob);
		} catch (err) {
			return undefined;
		}
	}

	protected releaseImages() {
		this.trackedImages.forEach((v) => {
			if (typeof v === "string")
				URL.revokeObjectURL(v);
		});
		this.trackedImages.clear();
	}
}

export class TauriBackend extends ImageTracking implements IBackend {
	public name = "Tauri";
	public cacheFileSrc: string = undefined!; // TODO: TAURI
	public readonly wsBaseAddress: string = undefined!; // TODO: TAURI

	private connections: Map<string, TauriBackendConnection> = new Map();

	constructor() {
		super();
		log("Using tauri backend");

		listen<TauriP2FWs>("ws", (ev) => {
			log("QintConnection: %o", ev);
			const msg = ev.payload;
			const con = this.connections.get(msg.con);
			if (con !== undefined) {
				con.onMsg?.(msg.msg);
			}
		});

		listen<TauriP2FClose>("ws_close", (ev) => {
			log("Closing event: %o", ev);
			const conId = ev.payload;
			const con = this.connections.get(conId);
			if (con !== undefined) {
				con.onClose?.();
				this.connections.delete(conId);
			}
		});
	}

	public createNewConnection(): IBackendConnection {
		const con = new TauriBackendConnection();
		this.connections.set(con.id, con);
		return con;
	}

	public close(): void {
		this.connections.forEach(con => con.close());
	}

	public async fetch(_cmd: string, _data: RequestInit): Promise<IFetchLike> {
		throw new Error("Not implemented");
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

	public async fetch_cache_image(req: ICacheFileRequest): Promise<string | undefined> {
		return await this.fetchImgInternal("get_cache_file", req, req.server);
	}

	public async peek_link(link: string): Promise<RustAnalyzeResult> {
		return await invoke<RustAnalyzeResult>("peek_link", { link });
	}

	public async get_audio_device_list(): Promise<IAudioDeviceList> {
		return await invoke<IAudioDeviceList>("get_audio_device_list");
	}

	public async identity_create(): Promise<ApiIdentity> {
		return await invoke<ApiIdentity>("identity_create");
	}

	public async identity_import(data: string): Promise<void> {
		await invoke("identity_import", { data });
	}

	public async identity_list(find: FindIdentity): Promise<ApiIdentity[]> {
		return await invoke<ApiIdentity[]>("identity_list", { find });
	}

	public async identity_update(id: string, update: UpdateIdentityOptions): Promise<void> {
		await invoke("identity_update", { id, update });
	}

	public async identity_delete(id: string): Promise<void> {
		await invoke("identity_delete", { id });
	}

	public async get_mutestate(): Promise<MuteStates> {
		return await invoke<MuteStates>("get_mutestate");
	}

	public async run_hotkey(action: HotkeyAction): Promise<void> {
		await invoke("run_hotkey", { action });
	}


	public async plugin_list(): Promise<string[]> {
		return await invoke<string[]>("plugin_list");
	}

	public async plugin_get(name: string): Promise<string> {
		return await invoke<string>("plugin_get", { name });
	}

	public async plugin_save(name: string, content: string): Promise<void> {
		await invoke("plugin_save", { name, content });
	}

	public async plugin_delete(name: string): Promise<void> {
		await invoke("plugin_delete", { name });
	}

	public async plugin_load(name: string): Promise<IPlugin> {
		const content = await this.plugin_get(name);
		// https://stackoverflow.com/a/67359410/2444047
		const dataUri = URL.createObjectURL(new Blob([content], { type: 'text/javascript' }));
		try {
			return await importFunc(dataUri);
		} finally {
			URL.revokeObjectURL(dataUri);
		}
	}
}

export class TauriBackendConnection extends ImageTracking implements IBackendConnection {
	public serverFileSrc: string = undefined!; // TODO: TAURI
	public readonly id: string;
	onMsg?: msgFn;
	onError?: errorFn;
	onClose?: closedFn;

	constructor() {
		super();
		this.id = createUuidV4();
	}

	public async connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void> {
		this.onMsg = onMsg;
		this.onError = onError;
		this.onClose = onClose;

		log("Creating message channel with %s", this.id);
		try {
			await invoke<TauriF2PCreate>("create_ws", { con: this.id });
		} catch (err: unknown) {
			this.onError?.(JSON.stringify(err));
		}
	}

	public async send(data: OutMsg): Promise<void> {
		await invoke<TauriF2PWs>("pass_ws_msg", { con: this.id, msg: data });
	}

	public async close(): Promise<void> {
		const id = this.id;
		log("closing %s", id);
		this.releaseImages();
		try {
			await invoke<TauriF2PClose>("close_ws", { con: id });
			log("closed %s", id);
		} catch (err) {
			log("Failed to close connection %s: %s", id, err);
		}
	}

	public async fetch(_cmd: string, _data: RequestInit): Promise<IFetchLike> {
		throw new Error("Not implemented");
	}

	public async fetch_image(req: IFileRequest): Promise<string | undefined> {
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
