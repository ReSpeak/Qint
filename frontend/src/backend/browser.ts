import type { ResultDetails } from "./ws";
import { assert, hasProperty, hexEncode, javaHash, PromiseParts } from "../util";
import type {
	AskReadResult,
	IBackend,
	IBackendConnection,
	ICacheFileRequest,
	IFileRequest,
	TransferResult,
	UploadFeature,
} from "./backend";
import {
	IInvokeConnection,
	InvokeArgs,
	InvokeBackend,
	InvokeBackendConnection,
} from "./invokeConnection";
import { guessName } from "../ui/specialized/uiRenderedText";
import { FiletransferManager, UploadFile } from "./filetransferManager";
import { pathJoin } from "../panel/fileUtil";
import debug from "debug";
import { ReturnCodeTracker } from "./returnCodeTracker";
import FileIO from "../ui/util/FileIO.svelte";
const log = debug("BROWSER-WS");

const IS_DEVSERVER = (import.meta as any).webpackHot;
const BASE_ADDRESS = IS_DEVSERVER ? "http://localhost:4422" : "";

type WsP2FMsg = { cmd: string; returnCode?: string; con?: string; msg?: any };

function urlToWebSocket(url: string): string {
	let path = url;
	if (!path.startsWith("http")) path = window.location.origin;
	if (!path.startsWith("http")) throw Error("Failed to get websocket path");
	return "ws" + path.substring(4);
}

class BrowserInvokeConnection implements IInvokeConnection {
	public name: string = "browser-ws";
	backend!: BrowserBackend;

	/** The url address prefix for websockets */
	private readonly wsBaseAddress: string = urlToWebSocket(BASE_ADDRESS);

	private socket?: WebSocket;
	private connecting?: Promise<void>;
	private curReturnCode = 0;
	private returnCodes = new Map<string, PromiseParts<any>>();

	constructor() {}

	private connect(): Promise<void> {
		log("Connecting");
		this.socket?.close();
		this.socket = new WebSocket(`${this.wsBaseAddress}/ws`);
		this.socket.onerror = (error) => console.error("browser-ws error", String(error));
		this.socket.onmessage = (evt) => {
			const msg = JSON.parse(evt.data) as WsP2FMsg;
			switch (msg.cmd) {
				case "loudness":
					this.backend.loudnessListener.call(msg.msg);
					break;
				case "ws_close": {
					log("Closing event: %o", msg);
					const conId = msg.con!;
					const con = this.backend.connections.get(conId);
					if (con !== undefined) {
						con.onClose?.();
						this.backend.connections.delete(conId);
					}
					break;
				}
				case "ws": {
					log("QintConnection: %o", msg);
					const con = this.backend.connections.get(msg.con!);
					if (con !== undefined) {
						con.onMsg?.(msg.msg);
					}
					break;
				}
				case "resp": {
					const ret = this.returnCodes.get(msg.returnCode!);
					if (ret !== undefined) {
						this.returnCodes.delete(msg.returnCode!);
						ret.resolve(msg.msg);
					}
					break;
				}
				case "resp_err": {
					const ret = this.returnCodes.get(msg.returnCode!);
					if (ret !== undefined) {
						this.returnCodes.delete(msg.returnCode!);
						ret.reject(msg.msg);
					}
					break;
				}
			}
		};
		let rejectFun: (reason?: any) => void;
		this.connecting = new Promise((resolve, reject) => {
			rejectFun = reject;
			this.socket!.onopen = () => {
				log("Connected");
				this.connecting = undefined;
				resolve();
			};
		});
		this.socket.onclose = () => {
			log("Closing");
			this.socket = undefined;

			// Reject all promises
			rejectFun("Websocket closed");
			for (const ret of this.returnCodes.values()) {
				ret.reject();
			}
			this.returnCodes.clear();

			// Close all connections
			for (const con of this.backend.connections.values()) {
				con.onClose?.();
			}
			this.backend.connections.clear();
		};
		return this.connecting;
	}

	public async invoke<T = void>(cmd: string, args?: InvokeArgs): Promise<T> {
		log("invoke " + cmd);
		if (this.socket === undefined) await this.connect();
		if (this.connecting !== undefined) await this.connecting;

		if (this.socket !== undefined) {
			const returnCode = this.curReturnCode.toString();
			log(`sending ${cmd} return code ${returnCode}`);
			this.curReturnCode = (this.curReturnCode + 1) % 65536;
			this.socket.send(JSON.stringify({ cmd, returnCode, args: args ?? {} }));
			return new Promise((resolve, reject) => {
				this.returnCodes.set(returnCode, { resolve, reject });
			});
		} else {
			return new Promise((_resolve, reject) => reject("Failed to connect websocket"));
		}
	}

	public createNewConnection(
		returnCodes: ReturnCodeTracker
	): IBackendConnection & InvokeBackendConnection {
		return new BrowserBackendConnection(this.backend, returnCodes);
	}
}

export class BrowserBackend extends InvokeBackend<BrowserInvokeConnection> implements IBackend {
	public readonly cacheFileSrc: string;

	/** The url address prefix for websockets */
	public readonly wsBaseAddress: string = urlToWebSocket(BASE_ADDRESS);
	public fileIo!: FileIO;

	constructor() {
		super(new BrowserInvokeConnection());
		this.inner.backend = this;
		log("Using browser-ws backend");
		this.cacheFileSrc = `${BASE_ADDRESS}/filecache`;
	}

	public fetch_cache_image(req: ICacheFileRequest): Promise<string> {
		assert(!req.server.includes("/"), "Server must be url-safe base64");
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
		} catch {
			return undefined;
		}
		if (files.length === 0) return undefined;
		const file0 = files[0];
		return { content: await file0.text(), name: file0.name };
	}
}

export class BrowserBackendConnection
	extends InvokeBackendConnection
	implements IBackendConnection
{
	public serverFileSrc: string;
	private readonly filetransferManager: FiletransferManager = new FiletransferManager(this);

	constructor(private browserBackend: BrowserBackend, private returnCodes: ReturnCodeTracker) {
		super(browserBackend.inner);
		this.serverFileSrc = `${BASE_ADDRESS}/con/${this.id}`;
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

	// Filetransfer

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
		this.browserBackend.fileIo.askDownload(`${src}?dl=${encodeURIComponent(finalName)}`);
		return { uploadPromise: Promise.resolve() };
	}

	private static readonly NoFilesSelected: ResultDetails = { description: "No files selected" };

	public async ask_upload(target: UploadFeature): Promise<TransferResult> {
		let files: FileList;
		const is_files = hasProperty(target, "Files");
		const multiple = is_files;
		try {
			files = await this.browserBackend.fileIo.askUpload(multiple);
		} catch {
			throw BrowserBackendConnection.NoFilesSelected;
		}

		if (!files || files.length == 0) throw BrowserBackendConnection.NoFilesSelected;

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
					returnCode,
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
			this.filetransferManager.uploadFiles(
				...[...files].map((file) => {
					return {
						data: file,
						channelId,
						path: pathJoin(path, file.name),
					};
				})
			);
			return { uploadPromise: Promise.resolve() }; // TODO
		}
	}

	// TODO Consider piping the array of existing values to this funtion
	// TODO Consider actually hasing the icon file
	private upload_feature_icon(file: File) {
		// eslint-disable-next-line prefer-const
		let number = javaHash(file.name) >>> 0; // Convert to unsigned 32-bit int
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
