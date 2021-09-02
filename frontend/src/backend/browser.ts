import { InMsg, OutMsg, ResultDetails } from "./ws";
import { createUuidV4, hasProperty, hexEncode, javaHash, PromiseParts } from "../util";
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
	LoudnessEvent,
	LoudnessUnsubscribe,
	msgFn,
	TransferResult,
	UpdateIdentityOptions,
	UploadFeature,
} from "./backend";
import { RustAnalyzeResult } from "../chat/previewAnalyzer";
import { ApiIdentity } from "../panel/settings/identity";
import { MuteStates } from "../connect/uiConnect";
import { HotkeyAction } from "../transientSettings";
import { importFunc, IPlugin } from "../plugins";
import FileIO from "../ui/util/FileIO.svelte";
import { guessName } from "../ui/specialized/uiRenderedText";
import { FiletransferManager, UploadFile } from "./filetransferManager";
import { pathJoin } from "../panel/fileUtil";
import { ReturnCodeTracker } from "./returnCodeTracker";

const IS_SNOWPACK = (import.meta as any).hot;
const BASE_ADDRESS = IS_SNOWPACK ? "http://localhost:4422" : "";

export class BrowserBackend implements IBackend {
	public name = "Browser";
	public readonly cacheFileSrc: string;

	/** The url address prefix for websockets */
	public readonly wsBaseAddress: string = urlToWebSocket(BASE_ADDRESS);
	public fileIo!: FileIO;

	constructor() {
		this.cacheFileSrc = `${BASE_ADDRESS}/filecache`;
	}

	public createNewConnection(returnCodes: ReturnCodeTracker): IBackendConnection {
		return new BrowserBackendConnection(this, returnCodes);
	}

	public close(): void {
		// Nothing to do here for browser backed since websockets get killed.
	}

	private fetch(cmd: string, data?: RequestInit): Promise<Response> {
		return fetch(`${BASE_ADDRESS}${cmd}`, data);
	}

	public async graphql<T = any>(
		query: string,
		variables?: Record<string, unknown>
	): Promise<{ data: T }> {
		const response = await this.fetch(`/db`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ query, variables }),
		});
		return await response.json();
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
		const response = await this.fetch("/settings");
		return await response.json();
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
		if (req.hash) {
			str += (hasQ ? "&" : "?") + "hash=" + encodeURIComponent(req.hash);
			hasQ = true;
		}
		return Promise.resolve(str);
	}

	public async ask_read_file(): Promise<AskReadResult | undefined> {
		let files: FileList;
		try {
			files = await this.fileIo.askUpload(false);
		} catch { return undefined; }
		if (files.length === 0) return undefined;
		const file0 = files[0];
		return { content: await file0.text(), name: file0.name };
	}

	public async peek_link(link: string): Promise<RustAnalyzeResult> {
		const response = await this.fetch(`/peek_link/${encodeURIComponent(link)}`);
		return await response.json();
	}

	public async get_audio_device_list(): Promise<IAudioDeviceList> {
		const response = await this.fetch("/audio/device_list");
		return await response.json();
	}

	public async identity_create(): Promise<ApiIdentity> {
		const response = await this.fetch("/ident/new", {
			method: "POST",
		});
		return await response.json();
	}

	public async identity_import(data: string): Promise<void> {
		await this.fetch("/ident/import", {
			method: "POST",
			body: data,
		});
	}

	public async identity_list(find: FindIdentity): Promise<ApiIdentity[]> {
		if (find == "All") {
			const response = await this.fetch("/ident/all");
			return (await response.json()) as ApiIdentity[];
		}
		throw Error("Not implemented");
	}

	public async identity_update(id: string, update: UpdateIdentityOptions): Promise<void> {
		await this.fetch(`/ident/${id}?name=${update.name}`, {
			method: "PUT",
		});
	}

	public async identity_delete(id: string): Promise<void> {
		await this.fetch(`/ident/${id}`, {
			method: "DELETE",
		});
	}

	public async get_mutestate(): Promise<MuteStates> {
		return await (await this.fetch("/mutestate")).json();
	}

	public async run_hotkey(action: HotkeyAction): Promise<void> {
		await this.fetch("/hotkey", {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify(action),
		});
	}

	public async plugin_list(): Promise<string[]> {
		const response = await this.fetch("/plugins");
		return await response.json();
	}

	public async plugin_get(name: string): Promise<string> {
		const response = await this.fetch(`/plugins/${name}`);
		return await response.text();
	}

	public async plugin_save(name: string, content: string): Promise<void> {
		await this.fetch(`/plugins/${name}`, {
			method: "PUT",
			body: content,
		});
	}

	public async plugin_delete(name: string): Promise<void> {
		await this.fetch(`/plugins/${name}`, {
			method: "DELETE",
		});
	}

	public plugin_load(name: string): Promise<IPlugin> {
		return importFunc(`${BASE_ADDRESS}/plugins/${name}`);
	}

	public get_markdown_transformer(): IMarkdownTransform {
		return new BrowserMarkdownTransform(`${this.wsBaseAddress}/render_md_service`);
	}

	public get_loudness_listener(callback: LoudnessEvent): LoudnessUnsubscribe {
		const loudnessSocket = new WebSocket(`${this.wsBaseAddress}/loudness`);
		loudnessSocket.binaryType = "arraybuffer";
		loudnessSocket.onmessage = (ev) => {
			const data = new DataView(ev.data);
			const loudness = data.getFloat64(0);
			const vad = data.getFloat32(8);
			callback([loudness, vad]);
		};
		return () => { loudnessSocket.close(); }
	}
}

export class BrowserBackendConnection implements IBackendConnection {
	public serverFileSrc: string;
	public readonly id: string;
	private socket?: WebSocket;
	private readonly filetransferManager: FiletransferManager = new FiletransferManager(this);
	constructor(
		private parent: BrowserBackend,
		private returnCodes: ReturnCodeTracker,
	) {
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

	public fetch(cmd: string, data: RequestInit): Promise<Response> {
		return fetch(`${this.serverFileSrc}${cmd}`, data);
	}

	public fetch_image(req: IFileRequest): Promise<string> {
		let str = `${this.serverFileSrc}/file/${req.channel}${req.path}`;
		let hasQ = false;
		if (req.cache) {
			str += (hasQ ? "&" : "?") + "cache=true";
			hasQ = true;
		}
		if (req.hash) {
			str += (hasQ ? "&" : "?") + "hash=" + encodeURIComponent(req.hash);
			hasQ = true;
		}
		return Promise.resolve(str);
	}

	public async upload_bytes(req: IFileRequest, data: Blob): Promise<TransferResult> {
		const [returnCode, request] = this.returnCodes.getNew();
		const uploadPromise = this.fetch(
			`/file/${req.channel}${req.path}?return_code=${returnCode}`,
			{
				method: "PUT",
				body: data,
			}
		);
		const details = await request;
		if (details) throw details;
		return { uploadPromise };
	}

	public async ask_download(req: IFileRequest): Promise<TransferResult> {
		const src = await this.fetch_image(req);
		const finalName = req.suggested_name ?? guessName(src) ?? "file";
		// TODO add return code ?
		this.parent.fileIo.askDownload(`${src}?dl=${encodeURIComponent(finalName)}`);
		return { uploadPromise: Promise.resolve() };
	}

	private static readonly NoFilesSelected: ResultDetails = { description: "No files selected" };

	public async ask_upload(target: UploadFeature): Promise<TransferResult> {
		let files: FileList;
		const is_files = hasProperty(target, "Files");
		const multiple = is_files;
		try {
			files = await this.parent.fileIo.askUpload(multiple);
		} catch { throw BrowserBackendConnection.NoFilesSelected; }

		if (!files || files.length == 0)
			throw BrowserBackendConnection.NoFilesSelected;

		if (target === "Avatar") {
			const [returnCode, request] = this.returnCodes.getNew();
			try {
				const [hash, data] = await this.upload_feature_avatar(files[0]);
				const uploadFile: UploadFile = {
					data,
					channelId: "0",
					path: "/avatar",
					returnCode,
				};
				const uploadPromise = this.filetransferManager.uploadSingleFile(uploadFile);
				const details = await request;
				if (details) throw details;
				return { uploadPromise, featureData: hash };
			} finally {
				this.returnCodes.reject(returnCode);
			}
		} else if (target === "Icon") {
			const [returnCode, request] = this.returnCodes.getNew();
			try {
				const file0 = files[0];
				const uploadFile = {
					data: file0,
					channelId: "0",
					path: this.upload_feature_icon(file0),
					returnCode
				};
				const uploadPromise = this.filetransferManager.uploadSingleFile(uploadFile);
				const details = await request;
				if (details) throw details;
				return { uploadPromise };
			} finally {
				this.returnCodes.reject(returnCode);
			}
		} else {
			const [channelId, path] = target.Files;
			this.filetransferManager.uploadFiles(...[...files].map((file) => {
				return {
					data: file,
					channelId,
					path: pathJoin(path, file.name),
				};
			}));
			return { uploadPromise: Promise.resolve() }; // TODO
		}
	}

	// TODO Consider piping the array of existing values to this funtion
	// TODO Consider actually hasing the icon file
	private upload_feature_icon(file: File) {
		// eslint-disable-next-line prefer-const
		let number = javaHash(file.name);
		// while (displayFiles.find((node) => node.name === name) !== undefined) {
		// 	number++;
		// 	number &= number; // Truncate to u32
		// }
		const name = "/icon_" + number;
		return name;
	}

	private async upload_feature_avatar(file: File): Promise<[string, BodyInit]> {
		// Use SHA-256 hash and fall back to file size if not available
		if (crypto.subtle) {
			const buffer = await file.arrayBuffer();
			const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
			return [hexEncode(new Uint8Array(hashBuffer)), buffer];
		}
		return [file.size.toString(), file];
	}
}

function urlToWebSocket(url: string): string {
	let path = url;
	if (!path.startsWith("http")) path = window.location.origin;
	if (!path.startsWith("http")) throw Error("Failed to get websocket path");
	return "ws" + path.substring(4);
}

class BrowserMarkdownTransform implements IMarkdownTransform {
	private mdRenderSocket: WebSocket | undefined;
	private promise: PromiseParts<string> & { p: Promise<string> } | undefined;

	constructor(url: string) {
		this.mdRenderSocket = new WebSocket(url);
		this.mdRenderSocket.onclose = () => {
			if (this.promise !== undefined) {
				const p = this.promise;
				this.promise = undefined;
				p.reject();
			}
			this.mdRenderSocket = undefined;
		};
		this.mdRenderSocket.onmessage = (ev) => {
			if (this.promise !== undefined) {
				const p = this.promise;
				this.promise = undefined;
				p.resolve(ev.data as string);
			}
		};
	}

	public write(md: string): Promise<string> {
		if (this.mdRenderSocket !== undefined) {
			if (this.promise === undefined) {
				let promisePart: PromiseParts<string> = undefined!;
				const p = new Promise<string>((resolve, reject) => {
					promisePart = { resolve, reject };
				});
				this.promise = { ...promisePart, p };
				this.mdRenderSocket.send(md);
			}
			return this.promise.p;
		} else {
			return Promise.reject();
		}
	}

	public close() {
		this.mdRenderSocket?.close();
		this.mdRenderSocket = undefined;
	}
}
