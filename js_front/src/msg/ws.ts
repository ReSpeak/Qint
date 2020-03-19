interface IMsg<T extends string> {
	_cmd: T;
}

// Out Messages
export type OutMsg = IMsgConnect;

interface IMsgConnect extends IMsg<"connect"> {
	address: string;
}

// In Messages
export type InMsg = IMsgBookAdd | IMsgBookChange | IMsgBookRemove | IMsgConnected;

interface IMsgBookAdd extends IMsg<"b_add"> {
	obj: any;
}

interface IMsgBookChange extends IMsg<"b_change"> {
	obj: any;
}

interface IMsgBookRemove extends IMsg<"b_remove"> {
	obj: any;
}

// tslint:disable-next-line: no-empty-interface
interface IMsgConnected extends IMsg<"connected"> {}
