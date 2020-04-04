// Out Messages
export type OutMsg = { Connect: OMsgConnect }
	| { SendMessage: OMsgSendMessage };

interface OMsgConnect {
	address: string;
	name: string;
	log_commands: boolean;
	log_packets: boolean;
	log_udp_packets: boolean;
	version: string;
}

interface OMsgSendMessage {
	target: string; // TODO
	message: string;
}

// In Messages
export type InMsg = { Error: string }
	| { TalkersChanged: [number, boolean][] }
	| { Events: InBookMsg[] };

interface IInMsg<T extends string> {
	_cmd: T;
}

type InBookMsg = IMsgBookAdd | IMsgBookChange | IMsgBookRemove | IMsgConnected;

interface IMsgBookAdd extends IInMsg<"b_add"> {
	obj: any;
}

interface IMsgBookChange extends IInMsg<"b_change"> {
	obj: any;
}

interface IMsgBookRemove extends IInMsg<"b_remove"> {
	obj: any;
}

// tslint:disable-next-line: no-empty-interface
interface IMsgConnected extends IInMsg<"connected"> {}
