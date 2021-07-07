import { IS_TAURI } from "../util";
import { BrowserBackend } from "./browser";
import { TauriBackend } from "./tauri";
import { InMsg, OutMsg } from "./ws";

export type errorFn = (err: string) => void;
export type msgFn = (msg: InMsg) => void;
export type closedFn = () => void;

export interface IBackend {
	readonly name: string;
	readonly cacheFileSrc: string;
	/** The url address prefix for websockets */
	readonly wsBaseAddress: string;
	createNewConnection(): IBackendConnection;
	fetch(cmd: string, data?: RequestInit): Promise<IFetchLike>;
	setTitle(name: string): void;
	setIcon(url: string | undefined): void;

	// common interface

	graphql<T = any>(query: string, variables?: Record<string, unknown>): Promise<{ data: T }>;

	get_settings(): Promise<Record<string, unknown>>;
	set_settings(diff: Record<string, unknown>): Promise<void>;
}

export interface IBackendConnection {
	readonly id: string;
	readonly serverFileSrc: string;
	send(data: OutMsg): void;
	connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void>;
	close(): void;
	fetch(cmd: string, data?: RequestInit): Promise<IFetchLike>;
}

export interface IFetchLike {
	json(): Promise<any>;
	text(): Promise<string>;
}

export const backend: IBackend = IS_TAURI ? new TauriBackend() : new BrowserBackend();
