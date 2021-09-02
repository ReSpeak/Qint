import { pathJoin } from "../panel/fileUtil";
import { ChannelId } from "../ts";
import { Writable, writable } from "svelte/store";

type IBackendConnection = { fetch: (cmd: string, data: RequestInit) => Promise<Response> };

export class FiletransferManager {
	private uploadQueue: UploadFile[] = [];
	private currentUploadTask: Promise<void> | undefined;
	public uploadState: Writable<number> = writable(0);

	constructor(private backend: IBackendConnection) {}

	public uploadFiles(...files: UploadFile[]): void {
		if (files.length === 0) return;
		this.uploadQueue.push(...files);
		this.uploadState.set(this.uploadQueue.length);
		if (this.currentUploadTask === undefined) {
			this.currentUploadTask = this.uploadTaskFn();
		}
	}

	public async uploadSingleFile(file: UploadFile): Promise<void> {
		let link = `/file${pathJoin(file.channelId, file.path)}`;
		if (file.returnCode) {
			link += "?return_code=" + file.returnCode;
		}
		await this.backend.fetch(link, {
			method: "PUT",
			body: file.data,
		});
	}

	private async uploadTaskFn(): Promise<void> {
		while (this.uploadQueue.length !== 0) {
			const file = this.uploadQueue.shift()!;
			await this.uploadSingleFile(file);
			this.uploadState.set(this.uploadQueue.length);
		}
		this.currentUploadTask = undefined;
	}
}

export interface UploadFile {
	data: BodyInit;
	channelId: ChannelId;
	path: string;
	returnCode?: string;
}
