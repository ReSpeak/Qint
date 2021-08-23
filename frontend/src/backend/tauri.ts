import { InMsg, OutMsg, ResultDetails } from "./ws";
import { createUuidV4, fnBroadcast } from "../util";
import {
	AskReadResult,
	closedFn,
	errorFn,
	FindIdentity,
	IAudioDeviceList,
	IBackend,
	IBackendConnection,
	ICacheFileRequest,
	IFileRequest,
	IMarkdownTransform,
	LoudnessData,
	LoudnessEvent,
	LoudnessUnsubscribe,
	msgFn,
	TransferResult,
	UpdateIdentityOptions,
	UploadFeature,
} from "./backend";
import { listen } from "@tauri-apps/api/event";
import debug from "debug";
import { invoke } from "@tauri-apps/api/tauri";
import { RustAnalyzeResult } from "../chat/previewAnalyzer";
import { ApiIdentity } from "../panel/settings/identity";
import { MuteStates } from "../connect/uiConnect";
import { HotkeyAction } from "../transientSettings";
import { importFunc, IPlugin } from "../plugins";
import { ReturnCodeTracker } from "./returnCodeTracker";
const log = debug("TAURI");

if (DEBUG_UTIL) {
	(window as any).invoke = invoke;
}

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

	public async fetchImgInternal(ep: string, req: IFileRequest, con: string): Promise<string | undefined> {
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
					existing: FileExistsAction.Error
				},
				cache: req.cache,
			});
			const buffer = new Uint8Array(response.data);
			const blob = new Blob([buffer], {
				type: response.mime,
			});
			return URL.createObjectURL(blob);
		} catch (err: any) {
			log("Failed to fetch image: %j", err);
			return undefined;
		}
	}

	public releaseImages() {
		this.trackedImages.forEach((v) => {
			if (typeof v === "string")
				URL.revokeObjectURL(v);
		});
		this.trackedImages.clear();
	}
}

export class TauriBackend implements IBackend {
	public name = "Tauri";
	private readonly imageTracker = new ImageTracking();
	private readonly loudnessListener = fnBroadcast<[LoudnessData]>();

	private connections: Map<string, TauriBackendConnection> = new Map();

	constructor() {
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

		listen<LoudnessData>("loudness", (ev) => {
			const data = ev.payload;
			this.loudnessListener.call(data);
		});

		listen<string[]>("tauri://file-drop", (data) => {
			console.log("tauri://file-drop", data);
		});
		listen<string[]>("tauri://file-drop-hover", (data) => {
			console.log("tauri://file-drop-hover", data);
		});
		listen<string[]>("tauri://file-drop-cancelled", (data) => {
			console.log("tauri://file-drop-cancelled", data);
		});
	}

	public createNewConnection(returnCodes: ReturnCodeTracker): IBackendConnection {
		const con = new TauriBackendConnection(returnCodes);
		this.connections.set(con.id, con);
		return con;
	}

	public close(): void {
		this.connections.forEach(con => con.close());
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
		return await this.imageTracker.fetchImgInternal("download_bytes_from_cache", req, req.server);
	}

	public async ask_read_file(): Promise<AskReadResult | undefined> {
		try {
			const [name, content] = await invoke<[string, string]>("read_file");
			return { name, content };
		} catch {
			return undefined;
		}
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

	public get_markdown_transformer(): IMarkdownTransform {
		return TauriMarkdownTransform.Instance;
	}

	public get_loudness_listener(callback: LoudnessEvent): LoudnessUnsubscribe {
		invoke("set_loudness_callback", { enabled: true });
		const unsub = this.loudnessListener.subscribe(callback);
		return () => {
			unsub();
			if (this.loudnessListener.isEmpty()) {
				invoke("set_loudness_callback", { enabled: false });
			}
		};
	}
}

declare let __TAURI_INVOKE_KEY__: number;

export class TauriBackendConnection implements IBackendConnection {
	public readonly id: string;
	private readonly imageTracker = new ImageTracking();
	onMsg?: msgFn;
	onError?: errorFn;
	onClose?: closedFn;

	constructor(
		private returnCodes: ReturnCodeTracker,
	) {
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

	public send(data: OutMsg): void {
		window.rpc.notify("pass_ws_msg", {
			__invokeKey: __TAURI_INVOKE_KEY__,
			callback: "",
			error: "",
			con: this.id,
			msg: data,
		});
	}

	public async close(): Promise<void> {
		const id = this.id;
		log("closing %s", id);
		this.imageTracker.releaseImages();
		try {
			await invoke<TauriF2PClose>("close_ws", { con: id });
			log("closed %s", id);
		} catch (err) {
			log("Failed to close connection %s: %j", id, err);
		}
	}

	public async fetch_image(req: IFileRequest): Promise<string | undefined> {
		console.log("Fetching", req);
		return await this.imageTracker.fetchImgInternal("download_bytes", req, this.id);
	}

	// Filetransfer

	public async upload_bytes(req: IFileRequest, data: Blob): Promise<TransferResult> {
		try {
			new Blob()
			await invoke<void>("upload_bytes", {
				req: {
					con: this.id,
					...req,
					existing: FileExistsAction.Overwrite
				},
				data: Array.from(new Uint8Array(await data.arrayBuffer()))
			});
			return { uploadPromise: Promise.resolve() };
		} catch (err: any) {
			log("upload_bytes error: %j", err);
			throw err as ResultDetails;
		}
	}

	public async ask_download(req: IFileRequest): Promise<TransferResult> {
		try {
			await invoke<void>("download_file", {
				req: {
					con: this.id,
					...req,
					existing: FileExistsAction.Error
				}
			});
			return { uploadPromise: Promise.resolve() };
		} catch (err: any) {
			log("ask_download error: %j", err);
			throw err as ResultDetails;
		}
	}

	public async ask_upload(feature: UploadFeature): Promise<TransferResult> {
		try {
			const featureData = await invoke<string | null>("upload_file", {
				feature,
				req: {
					con: this.id,
					channel_password: "", // TODO,
					existing: FileExistsAction.Error,
				}
			}) ?? undefined;
			return { uploadPromise: Promise.resolve(), featureData };
		} catch (err: any) {
			log("ask_upload error: %j", err);
			throw err as ResultDetails;
		}
	}
}

function reqAsKey(req: IFileRequest | ICacheFileRequest): string {
	return `${req.hash}:${req.channel}/${req.path}`;
}
interface GetFileResponse {
	data: ArrayLike<number>;
	mime: string | undefined;
}

enum FileExistsAction {
	Error = "Error",
	Overwrite = "Overwrite",
	Resume = "Resume",
}

class TauriMarkdownTransform implements IMarkdownTransform {
	public static Instance: IMarkdownTransform = new TauriMarkdownTransform();
	public write(md: string) { return invoke<string>("markdown", { md }); }
	public close() { }
}
