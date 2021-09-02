import { RustAnalyzeResult } from "../chat/previewAnalyzer";
import { MuteStates } from "../connect/uiConnect";
import { ApiIdentity } from "../panel/settings/identity";
import { Uid } from "../ts";
import { IS_TAURI } from "../util";
//import { BrowserBackend } from "./browser";
import { BrowserBackend } from "./browser2";
import { TauriBackend } from "./tauri";
import { InMsg, OutMsg } from "./ws";
import { HotkeyAction } from "../transientSettings";
import { IPlugin } from "../plugins";
import { ReturnCodeTracker } from "./returnCodeTracker";

export const backend: IBackend = IS_TAURI ? new TauriBackend() : new BrowserBackend();

export type errorFn = (err: string) => void;
export type msgFn = (msg: InMsg) => void;
export type closedFn = () => void;

export interface IBackend {
	readonly name: string;
	createNewConnection(returnCodes: ReturnCodeTracker): IBackendConnection;
	close(): void;
	setTitle(name: string): void;
	setIcon(url: string | undefined): void;

	// common interface

	graphql<T = any>(query: string, variables?: Record<string, unknown>): Promise<{ data: T }>;

	get_settings(): Promise<Record<string, unknown>>;
	set_settings(diff: Record<string, unknown>): Promise<void>;
	fetch_cache_image(img: ICacheFileRequest): Promise<string | undefined>;
	ask_read_file(): Promise<AskReadResult | undefined>;
	peek_link(link: string): Promise<RustAnalyzeResult>;
	get_audio_device_list(): Promise<IAudioDeviceList>;
	identity_create(): Promise<ApiIdentity>;
	identity_import(data: string): Promise<void>;
	identity_list(find: FindIdentity): Promise<ApiIdentity[]>;
	identity_update(id: string, update: UpdateIdentityOptions): Promise<void>;
	identity_delete(id: string): Promise<void>;
	get_mutestate(): Promise<MuteStates>;
	run_hotkey(action: HotkeyAction): Promise<void>;
	plugin_list(): Promise<string[]>;
	plugin_get(name: string): Promise<string>;
	plugin_save(name: string, content: string): Promise<void>;
	plugin_delete(name: string): Promise<void>;
	plugin_load(name: string): Promise<IPlugin>;
	get_markdown_transformer(): IMarkdownTransform;
	get_loudness_listener(callback: LoudnessEvent): LoudnessUnsubscribe;
}

export interface IBackendConnection {
	readonly id: string;
	send(data: OutMsg): void;
	connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void>;
	close(): void;

	fetch_image(img: IFileRequest): Promise<string | undefined>;

	/**
	 * @throws {ResultDetails} On ts error
	*/
	upload_bytes(req: IFileRequest, data: Blob): Promise<TransferResult>;

	/**
	 * Does *not* allow multiple uploads (for now).
	 * @throws {ResultDetails} On ts error
	*/
	ask_download(req: IFileRequest): Promise<TransferResult>;

	/**
	 * Multiple uploads for:
	 * - ❌ Upload Avatar
	 * - ❌ Upload Icon
	 * - ✅ Upload to folder
	 * 	- Returns: {string}
	 * @throws {ResultDetails} On ts error
	*/
	ask_upload(target: UploadFeature): Promise<TransferResult>;
}

export interface IFileRequest {
	channel: string;
	path: string;
	hash?: string;
	cache: boolean;
	suggested_name?: string;
}

export interface ICacheFileRequest extends IFileRequest {
	server: string;
}

export type UploadFeature =
	{ Files: [channel: string, path: string] }
	| "Avatar"
	| "Icon";

export interface AskReadResult {
	name: string;
	content: string;
}

export interface TransferResult {
	uploadPromise: Promise<any>;
	featureData?: string;
}

export const enum ImageProvider {
	Server,
	Cache,
}

export interface IAudioDeviceList {
	capture: string[];
	playback: string[];
}

export type FindIdentity =
	"All"
	| { ById: number }
	| { ByUid: Uid }
	| { ByName: string };

export interface UpdateIdentityOptions {
	name?: string;
}

export interface IMarkdownTransform {
	write(md: string): Promise<string>;
	close(): void;
}

export type LoudnessData = [loudness: number, vad: number];
export type LoudnessEvent = (ev: LoudnessData) => void;
export type LoudnessUnsubscribe = () => void;
