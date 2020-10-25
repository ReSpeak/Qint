import { BrowserBackend } from "./browser";
import { TauriBackend } from "./tauri";
import { InMsg, OutMsg } from "./ws";
import { IS_TAURI } from "../util";

export type errorFn = (err: string) => void;
export type msgFn = (msg: InMsg) => void;
export type closedFn = () => void;

export interface IBackend {
	cacheFileSrc: string;
	createNewConnection(): IBackendConnection;
	fetch(cmd: string, data?: RequestInit): Promise<IFetchLike>;
	graphql<T = any>(query: string, variables?: object): Promise<{ data: T }>;
	setTitle(name: string): void;
}

export interface IBackendConnection {
	id: string;
	serverFileSrc: string;
	send(data: OutMsg): void;
	connect(
		onMsg: msgFn,
		onError: errorFn,
		onClose: closedFn): Promise<void>;
	close(): void;
}

export interface IFetchLike {
	json(): Promise<any>;
	text(): Promise<string>;
}

export const backend: IBackend = /*IS_TAURI ? new TauriBackend() :*/ new BrowserBackend();
