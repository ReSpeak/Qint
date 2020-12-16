import { IMsgFileList, IMsgFileListPart, IMsgFileInfo } from "./book_events";
import { datetimeDeserialize } from "./util";
import moment, { Moment } from "moment";
import { pathSplit } from "./panel/fileUtil";
import debug from "debug";
const log = debug("FILECACHE");

export class FileTreeCache {
	public isLoading: boolean = false;
	public root: FileTreeFolder = new FileTreeFolder("", unknownDate);

	public applyFileList(fileList: IMsgFileList) {
		fileList.FileList.forEach(m => this.updateCache(m));
		log("File list %o", this)
		return this;
	}

	public applyFileInfo(fileInfo: IMsgFileInfo) {
		throw Error("not impl");
		//return this;
	}

	private static getPath(entry: IMsgFileListPart): string[] {
		return pathSplit(entry.channelId, entry.path, entry.name);
	}

	public updateCache(entry: IMsgFileListPart) {
		this.root.updateCachePath(FileTreeCache.getPath(entry), entry);
	}

	public get(path: string[], folderOnly: true): FileTreeFolder | null;
	public get(path: string[], folderOnly: boolean): FileTreeNode | null;
	public get(path: string[], folderOnly: boolean = true): FileTreeNode | null {
		return this.root.get(path, folderOnly);
	}

	public clear(path: string[] = []) {
		log("clearing %o", path);
		this.root.clear(path);
	}
}

export type FileTreeNode = FileTreeFile | FileTreeFolder;

class FileTreeFile {
	public isFile: true = true;
	public name: string;
	public size: number;
	public lastModified: Moment;

	constructor(entry: IMsgFileListPart) {
		this.name = entry.name;
		this.size = Number(entry.size);
		this.lastModified = datetimeDeserialize(entry.dateTime).local();
	}

}

const unknownDate = moment(0);
export const enum FolderState {
	Dummy,
	Loading,
	Loaded
}

class FileTreeFolder {
	public isFile: false = false;
	public children?: Map<string, FileTreeNode>;
	public contentLoaded: FolderState = FolderState.Dummy;

	constructor(
		public name: string,
		public lastModified: Moment,
	) {
	}

	private static createCacheDummy(name: string) {
		return new FileTreeFolder(
			name,
			unknownDate
		);
	}

	private static createFromEntry(entry: IMsgFileListPart) {
		return new FileTreeFolder(
			entry.name,
			datetimeDeserialize(entry.dateTime)
		);
	}

	public updateCachePath(path: string[], entry: IMsgFileListPart) {
		let [part, ...rest] = path;
		if (this.children === undefined)
			this.children = new Map();
		if (rest.length === 0) {
			this.contentLoaded = FolderState.Loading;
			if (entry.isFile) {
				this.children.set(part, new FileTreeFile(entry));
			} else {
				this.children.set(part, FileTreeFolder.createFromEntry(entry));
			}
		} else {
			let childFolder = this.children.get(part);
			if (childFolder === undefined || childFolder.isFile) {
				childFolder = FileTreeFolder.createCacheDummy(part);
				this.children.set(part, childFolder);
			}
			childFolder.updateCachePath(rest, entry);
		}
	}

	public get(path: string[], folderOnly: boolean): FileTreeNode | null {
		if (this.children === undefined) return null;
		let [part, ...rest] = path;
		let child = this.children.get(part);
		if (child === undefined) return null;
		if (rest.length === 0) {
			if (!child.isFile || !folderOnly)
				return child;
			return null;
		} else {
			if (!child.isFile)
				return child.get(rest, folderOnly);
			return null;
		}
	}

	public clear(path: string[]): void {
		if (this.children === undefined) return;
		if (path.length === 0) {
			this.children?.clear();
			this.contentLoaded = FolderState.Dummy;
		} else {
			let [part, ...rest] = path;
			let child = this.children.get(part);
			if (child === undefined || child.isFile) return;
			child.clear(rest);
		}
	}
}
