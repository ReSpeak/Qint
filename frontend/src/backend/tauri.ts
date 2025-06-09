import { InMsg, ResultDetails } from "./ws";
import {
	AskReadResult,
	IBackend,
	IBackendConnection,
	ICacheFileRequest,
	IFileRequest,
	LoudnessData,
	TransferResult,
	UploadFeature,
} from "./backend";
import { listen } from "@tauri-apps/api/event";
import debug from "debug";
import { invoke } from "@tauri-apps/api/core";
import { ReturnCodeTracker } from "./returnCodeTracker";
import {
	IInvokeConnection,
	InvokeArgs,
	InvokeBackend,
	InvokeBackendConnection,
} from "./invokeConnection";
const log = debug("TAURI");

if (typeof DEBUG_UTIL !== "undefined") {
	(window as any).invoke = invoke;
}

type TauriP2FWs = { con: string; msg: InMsg };
type TauriP2FClose = string;

class ImageTracking {
	private trackedImages: Map<string, string | undefined | Promise<string | undefined>> =
		new Map();

	public async fetchImgInternal(
		ep: string,
		req: IFileRequest,
		con: string
	): Promise<string | undefined> {
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

	private async fetchImgTask(
		ep: string,
		req: IFileRequest,
		con: string
	): Promise<string | undefined> {
		try {
			const response = await invoke<GetFileResponse>(ep, {
				req: {
					con,
					channel: req.channel,
					path: req.path,
					hash: req.hash,
					existing: FileExistsAction.Error,
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
			if (typeof v === "string") URL.revokeObjectURL(v);
		});
		this.trackedImages.clear();
	}
}

class TauriInvokeConnection implements IInvokeConnection {
	public name: string = "tauri";
	backend!: TauriBackend;

	constructor() {}

	public async invoke<T = void>(cmd: string, args?: InvokeArgs): Promise<T> {
		return invoke<T>(cmd, args);
	}

	public createNewConnection(
		_returnCodes: ReturnCodeTracker
	): IBackendConnection & InvokeBackendConnection {
		return new TauriBackendConnection(this.backend);
	}
}

export class TauriBackend extends InvokeBackend<TauriInvokeConnection> implements IBackend {
	private readonly imageTracker = new ImageTracking();

	constructor() {
		super(new TauriInvokeConnection());
		this.inner.backend = this;
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

	public async fetch_cache_image(req: ICacheFileRequest): Promise<string | undefined> {
		return await this.imageTracker.fetchImgInternal(
			"download_bytes_from_cache",
			req,
			req.server
		);
	}

	public async ask_read_file(): Promise<AskReadResult | undefined> {
		try {
			const [name, content] = await invoke<[string, string]>("read_file");
			return { name, content };
		} catch {
			return undefined;
		}
	}
}

export class TauriBackendConnection extends InvokeBackendConnection implements IBackendConnection {
	private readonly imageTracker = new ImageTracking();

	constructor(tauriBackend: TauriBackend) {
		super(tauriBackend.inner);
	}

	public async fetch_image(req: IFileRequest): Promise<string | undefined> {
		console.log("Fetching", req);
		return await this.imageTracker.fetchImgInternal("download_bytes", req, this.id);
	}

	// Filetransfer

	public async upload_bytes(req: IFileRequest, data: Blob): Promise<TransferResult> {
		try {
			new Blob();
			await invoke<void>("upload_bytes", {
				req: {
					con: this.id,
					...req,
					existing: FileExistsAction.Overwrite,
				},
				data: Array.from(new Uint8Array(await data.arrayBuffer())),
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
					existing: FileExistsAction.Error,
				},
			});
			return { uploadPromise: Promise.resolve() };
		} catch (err: any) {
			log("ask_download error: %j", err);
			throw err as ResultDetails;
		}
	}

	public async ask_upload(feature: UploadFeature): Promise<TransferResult> {
		try {
			const featureData =
				(await invoke<string | null>("upload_file", {
					feature,
					req: {
						con: this.id,
						channel_password: "", // TODO,
						existing: FileExistsAction.Error,
					},
				})) ?? undefined;
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
