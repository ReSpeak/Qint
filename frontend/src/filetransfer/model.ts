interface IFileSource {
	readonly type: string;
}

class LocalFiles implements IFileSource {
	get type(): "local" {
		return "local";
	}

	constructor(public filePaths: string[]) {}
}
