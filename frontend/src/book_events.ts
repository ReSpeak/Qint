import { ChannelGroupId, ChannelId, ClientDbId, ClientId, ClientType, EccKeyPubP256, IconId, IpAddr, MaxClients, ServerGroupId, SocketAddr, TalkPowerRequest, Uid } from "./ts";
import { datetimeDeserialize, durationDeserialize } from "./util";
import { Duration, Moment } from "moment";
import { ChannelBase, ClientBase, ServerBase, ServerGroupBase, ChannelGroupBase } from "./bookBase";

// Enums

export enum PermissionType {
	ServerGroup = "ServerGroup",
	GlobalClient = "GlobalClient",
	Channel = "Channel",
	ChannelGroup = "ChannelGroup",
	ChannelClient = "ChannelClient",
}

export enum TextMessageTargetMode {
	Unknown = "Unknown",
	Client = "Client",
	Channel = "Channel",
	Server = "Server",
}

export enum HostMessageMode {
	None = "None",
	Log = "Log",
	Modal = "Modal",
	Modalquit = "Modalquit",
}

export enum HostBannerMode {
	NoAdjust = "NoAdjust",
	AdjustIgnoreAspect = "AdjustIgnoreAspect",
	AdjustKeepAspect = "AdjustKeepAspect",
}

export enum Codec {
	SpeexNarrowband = "SpeexNarrowband",
	SpeexWideband = "SpeexWideband",
	SpeexUltrawideband = "SpeexUltrawideband",
	CeltMono = "CeltMono",
	OpusVoice = "OpusVoice",
	OpusMusic = "OpusMusic",
}

export enum CodecEncryptionMode {
	PerChannel = "PerChannel",
	ForcedOff = "ForcedOff",
	ForcedOn = "ForcedOn",
}

export enum Reason {
	None = "None",
	Moved = "Moved",
	Subscription = "Subscription",
	LostConnection = "LostConnection",
	KickChannel = "KickChannel",
	KickServer = "KickServer",
	KickServerBan = "KickServerBan",
	Serverstop = "Serverstop",
	Clientdisconnect = "Clientdisconnect",
	Channelupdate = "Channelupdate",
	Channeledit = "Channeledit",
	ClientdisconnectServerShutdown = "ClientdisconnectServerShutdown",
}

export enum GroupNamingMode {
	None = "None",
	Before = "Before",
	After = "After",
}

export enum GroupType {
	Template = "Template",
	Regular = "Regular",
	Query = "Query",
}

export enum LicenseType {
	NoLicense = "NoLicense",
	Offline = "Offline",
	Sdk = "Sdk",
	SdkOffline = "SdkOffline",
	Npl = "Npl",
	Athp = "Athp",
	Aal = "Aal",
	Default = "Default",
	Gamer = "Gamer",
	Sponsorship = "Sponsorship",
	Commercial = "Commercial",
}

export enum ChannelType {
	Permanent = "Permanent",
	SemiPermanent = "SemiPermanent",
	Temporary = "Temporary",
}

export enum TokenType {
	ServerGroup = "ServerGroup",
	ChannelGroup = "ChannelGroup",
}

export enum PluginTargetMode {
	CurrentChannel = "CurrentChannel",
	Server = "Server",
	Client = "Client",
	CurrentChannelSubsribedClients = "CurrentChannelSubsribedClients",
}

export enum LogLevel {
	Error = "Error",
	Warning = "Warning",
	Debug = "Debug",
	Info = "Info",
}

export enum ChannelPermissionHint {
	Join = "Join",
	Modify = "Modify",
	ForceDelete = "ForceDelete",
	Delete = "Delete",
	Subscribe = "Subscribe",
	ViewDescription = "ViewDescription",
	FileUpload = "FileUpload",
	FileDownload = "FileDownload",
	FileDelete = "FileDelete",
	FileRename = "FileRename",
	FileBrowse = "FileBrowse",
	FileDirectoryCreate = "FileDirectoryCreate",
	ModifyPermissions = "ModifyPermissions",
}

export enum ClientPermissionHint {
	KickServer = "KickServer",
	KickChannel = "KickChannel",
	Ban = "Ban",
	MoveClient = "MoveClient",
	PrivateMessage = "PrivateMessage",
	Poke = "Poke",
	Whisper = "Whisper",
	Complain = "Complain",
	ModifyPermissions = "ModifyPermissions",
}


// Structs

export class Channel extends ChannelBase {
	public readonly description!: string;
	public readonly id!: ChannelId;
	public readonly guid!: string | null;
	public readonly parent!: ChannelId;
	public readonly name!: string;
	public readonly topic!: string | null;
	public readonly codec!: Codec;
	public readonly codecQuality!: number | null;
	public readonly maxClients!: MaxClients | null;
	public readonly maxFamilyClients!: MaxClients | null;
	public readonly order!: ChannelId;
	public readonly channelType!: ChannelType;
	public readonly isDefault!: boolean | null;
	public readonly hasPassword!: boolean | null;
	public readonly codecLatencyFactor!: number | null;
	public readonly isUnencrypted!: boolean | null;
	public readonly deleteDelay!: Duration | null;
	public readonly neededTalkPower!: number | null;
	public readonly forcedSilence!: boolean;
	public readonly phoneticName!: string | null;
	public readonly icon!: IconId | null;
	public readonly isPrivate!: boolean | null;
	public readonly subscribed!: boolean;
	public readonly permissionHints!: ChannelPermissionHint | null;

	public update(obj: Partial<this>): this {
		if ("deleteDelay" in obj && obj.deleteDelay)
			Object.assign(obj, { deleteDelay: durationDeserialize((obj as any).deleteDelay) });
		Object.assign(this, obj);
		return this;
	}
}

export class Client extends ClientBase {
	public readonly version!: string;
	public readonly versionSign!: string | null;
	public readonly platform!: string;
	public readonly loginName!: string | null;
	public readonly created!: Moment;
	public readonly lastConnected!: Moment;
	public readonly connectionsTotal!: number;
	public readonly bytesUploadedMonth!: string;
	public readonly bytesDownloadedMonth!: string;
	public readonly bytesUploadedTotal!: string;
	public readonly bytesDownloadedTotal!: string;
	public readonly ping!: Duration | null;
	public readonly pingDeviation!: Duration | null;
	public readonly connectedTime!: Duration | null;
	public readonly clientAddress!: SocketAddr | null;
	public readonly packetsSentSpeech!: string | null;
	public readonly packetsSentKeepalive!: string | null;
	public readonly packetsSentControl!: string | null;
	public readonly bytesSentSpeech!: string | null;
	public readonly bytesSentKeepalive!: string | null;
	public readonly bytesSentControl!: string | null;
	public readonly packetsReceivedSpeech!: string | null;
	public readonly packetsReceivedKeepalive!: string | null;
	public readonly packetsReceivedControl!: string | null;
	public readonly bytesReceivedSpeech!: string | null;
	public readonly bytesReceivedKeepalive!: string | null;
	public readonly bytesReceivedControl!: string | null;
	public readonly serverToClientPacketlossSpeech!: number | null;
	public readonly serverToClientPacketlossKeepalive!: number | null;
	public readonly serverToClientPacketlossControl!: number | null;
	public readonly serverToClientPacketlossTotal!: number | null;
	public readonly clientToServerPacketlossSpeech!: number;
	public readonly clientToServerPacketlossKeepalive!: number;
	public readonly clientToServerPacketlossControl!: number;
	public readonly clientToServerPacketlossTotal!: number;
	public readonly bandwidthSentLastSecondSpeech!: string | null;
	public readonly bandwidthSentLastSecondKeepalive!: string | null;
	public readonly bandwidthSentLastSecondControl!: string | null;
	public readonly bandwidthSentLastMinuteSpeech!: string | null;
	public readonly bandwidthSentLastMinuteKeepalive!: string | null;
	public readonly bandwidthSentLastMinuteControl!: string | null;
	public readonly bandwidthReceivedLastSecondSpeech!: string | null;
	public readonly bandwidthReceivedLastSecondKeepalive!: string | null;
	public readonly bandwidthReceivedLastSecondControl!: string | null;
	public readonly bandwidthReceivedLastMinuteSpeech!: string | null;
	public readonly bandwidthReceivedLastMinuteKeepalive!: string | null;
	public readonly bandwidthReceivedLastMinuteControl!: string | null;
	public readonly filetransferBandwidthSent!: string | null;
	public readonly filetransferBandwidthReceived!: string | null;
	public readonly idleTime!: Duration;
	public readonly id!: ClientId;
	public readonly channel!: ChannelId;
	public readonly uid!: Uid | null;
	public readonly name!: string;
	public readonly inputMuted!: boolean;
	public readonly outputMuted!: boolean;
	public readonly outputOnlyMuted!: boolean;
	public readonly inputHardwareEnabled!: boolean;
	public readonly outputHardwareEnabled!: boolean;
	public readonly talkPowerGranted!: boolean;
	public readonly metadata!: string;
	public readonly isRecording!: boolean;
	public readonly databaseId!: ClientDbId;
	public readonly channelGroup!: ChannelGroupId;
	public readonly serverGroups!: ServerGroupId[];
	public readonly awayMessage!: string | null;
	public readonly clientType!: ClientType;
	public readonly avatarHash!: string;
	public readonly talkPower!: number;
	public readonly talkPowerRequest!: TalkPowerRequest | null;
	public readonly description!: string;
	public readonly isPrioritySpeaker!: boolean;
	public readonly unreadMessages!: number;
	public readonly phoneticName!: string;
	public readonly neededServerqueryViewPower!: number;
	public readonly icon!: IconId;
	public readonly isChannelCommander!: boolean;
	public readonly countryCode!: string;
	public readonly inheritedChannelGroupFromChannel!: ChannelId;
	public readonly badges!: string;
	public readonly permissionHints!: ClientPermissionHint | null;

	public update(obj: Partial<this>): this {
		if ("created" in obj && obj.created)
			Object.assign(obj, { created: datetimeDeserialize((obj as any).created) });
		if ("lastConnected" in obj && obj.lastConnected)
			Object.assign(obj, { lastConnected: datetimeDeserialize((obj as any).lastConnected) });
		if ("ping" in obj && obj.ping)
			Object.assign(obj, { ping: durationDeserialize((obj as any).ping) });
		if ("pingDeviation" in obj && obj.pingDeviation)
			Object.assign(obj, { pingDeviation: durationDeserialize((obj as any).pingDeviation) });
		if ("connectedTime" in obj && obj.connectedTime)
			Object.assign(obj, { connectedTime: durationDeserialize((obj as any).connectedTime) });
		if ("idleTime" in obj && obj.idleTime)
			Object.assign(obj, { idleTime: durationDeserialize((obj as any).idleTime) });
		Object.assign(this, obj);
		return this;
	}
}

export class Server extends ServerBase {
	public readonly uptime!: Duration;
	public readonly hasPassword!: boolean;
	public readonly defaultChannelAdminGroup!: ChannelGroupId;
	public readonly maxDownloadBandwidthTotal!: string;
	public readonly maxUploadBandwidthTotal!: string;
	public readonly complainAutobanCount!: number;
	public readonly complainAutobanTime!: Duration;
	public readonly complainRemoveTime!: Duration;
	public readonly minClientsInChannelBeforeForcedSilence!: number;
	public readonly antifloodPointsTickReduce!: number;
	public readonly antifloodPointsToCommandBlock!: number;
	public readonly antifloodPointsToIpBlock!: number;
	public readonly antifloodPointsToPluginBlock!: number;
	public readonly connectionCountTotal!: string;
	public readonly channelCount!: string;
	public readonly clientCount!: number;
	public readonly queryCountTotal!: string;
	public readonly queryCount!: number;
	public readonly downloadQuota!: string;
	public readonly uploadQuota!: string;
	public readonly bytesDownloadedMonth!: string;
	public readonly bytesUploadedMonth!: string;
	public readonly bytesDownloadedTotal!: string;
	public readonly bytesUploadedTotal!: string;
	public readonly port!: number;
	public readonly autostart!: boolean;
	public readonly machineId!: string;
	public readonly neededIdentitySecurityLevel!: number;
	public readonly logClient!: boolean;
	public readonly logQuery!: boolean;
	public readonly logChannel!: boolean;
	public readonly logPermissions!: boolean;
	public readonly logServer!: boolean;
	public readonly logFiletransfer!: boolean;
	public readonly minClientVersion!: Moment;
	public readonly reservedSlots!: number;
	public readonly totalPacketlossSpeech!: number;
	public readonly totalPacketlossKeepalive!: number;
	public readonly totalPacketlossControl!: number;
	public readonly totalPacketloss!: number;
	public readonly totalPing!: Duration;
	public readonly weblistEnabled!: boolean;
	public readonly minAndroidVersion!: Moment;
	public readonly minIosVersion!: Moment;
	public readonly filetransferBandwidthSent!: string;
	public readonly filetransferBandwidthReceived!: string;
	public readonly filetransferBytesSentTotal!: string;
	public readonly filetransferBytesReceivedTotal!: string;
	public readonly packetsSentTotal!: string;
	public readonly bytesSentTotal!: string;
	public readonly packetsReceivedTotal!: string;
	public readonly bytesReceivedTotal!: string;
	public readonly bandwidthSentLastSecondTotal!: string;
	public readonly bandwidthSentLastMinuteTotal!: string;
	public readonly bandwidthReceivedLastSecondTotal!: string;
	public readonly bandwidthReceivedLastMinuteTotal!: string;
	public readonly connectedTime!: Duration;
	public readonly packetlossTotal!: number;
	public readonly ping!: Duration;
	public readonly publicKey!: EccKeyPubP256;
	public readonly id!: string;
	public readonly name!: string;
	public readonly nickname!: string | null;
	public readonly welcomeMessage!: string;
	public readonly platform!: string;
	public readonly version!: string;
	public readonly maxClients!: number;
	public readonly created!: Moment;
	public readonly codecEncryptionMode!: CodecEncryptionMode;
	public readonly hostmessage!: string;
	public readonly hostmessageMode!: HostMessageMode;
	public readonly defaultServerGroup!: ServerGroupId;
	public readonly defaultChannelGroup!: ChannelGroupId;
	public readonly hostbannerUrl!: string;
	public readonly hostbannerGfxUrl!: string;
	public readonly hostbannerGfxInterval!: Duration;
	public readonly prioritySpeakerDimmModificator!: number;
	public readonly hostbuttonTooltip!: string;
	public readonly hostbuttonUrl!: string;
	public readonly hostbuttonGfxUrl!: string;
	public readonly phoneticName!: string;
	public readonly icon!: IconId;
	public readonly ips!: IpAddr[];
	public readonly askForPrivilegekey!: boolean;
	public readonly hostbannerMode!: HostBannerMode;
	public readonly tempChannelDefaultDeleteDelay!: Duration;
	public readonly protocolVersion!: number;
	public readonly license!: LicenseType;
	public readonly ownClient!: ClientId;

	public update(obj: Partial<this>): this {
		if ("uptime" in obj && obj.uptime)
			Object.assign(obj, { uptime: durationDeserialize((obj as any).uptime) });
		if ("complainAutobanTime" in obj && obj.complainAutobanTime)
			Object.assign(obj, { complainAutobanTime: durationDeserialize((obj as any).complainAutobanTime) });
		if ("complainRemoveTime" in obj && obj.complainRemoveTime)
			Object.assign(obj, { complainRemoveTime: durationDeserialize((obj as any).complainRemoveTime) });
		if ("minClientVersion" in obj && obj.minClientVersion)
			Object.assign(obj, { minClientVersion: datetimeDeserialize((obj as any).minClientVersion) });
		if ("totalPing" in obj && obj.totalPing)
			Object.assign(obj, { totalPing: durationDeserialize((obj as any).totalPing) });
		if ("minAndroidVersion" in obj && obj.minAndroidVersion)
			Object.assign(obj, { minAndroidVersion: datetimeDeserialize((obj as any).minAndroidVersion) });
		if ("minIosVersion" in obj && obj.minIosVersion)
			Object.assign(obj, { minIosVersion: datetimeDeserialize((obj as any).minIosVersion) });
		if ("connectedTime" in obj && obj.connectedTime)
			Object.assign(obj, { connectedTime: durationDeserialize((obj as any).connectedTime) });
		if ("ping" in obj && obj.ping)
			Object.assign(obj, { ping: durationDeserialize((obj as any).ping) });
		if ("created" in obj && obj.created)
			Object.assign(obj, { created: datetimeDeserialize((obj as any).created) });
		if ("hostbannerGfxInterval" in obj && obj.hostbannerGfxInterval)
			Object.assign(obj, { hostbannerGfxInterval: durationDeserialize((obj as any).hostbannerGfxInterval) });
		if ("tempChannelDefaultDeleteDelay" in obj && obj.tempChannelDefaultDeleteDelay)
			Object.assign(obj, { tempChannelDefaultDeleteDelay: durationDeserialize((obj as any).tempChannelDefaultDeleteDelay) });
		Object.assign(this, obj);
		return this;
	}
}

export class ServerGroup extends ServerGroupBase {
	public readonly id!: ServerGroupId;
	public readonly name!: string;
	public readonly groupType!: GroupType;
	public readonly icon!: IconId;
	public readonly isPermanent!: boolean;
	public readonly sortId!: number;
	public readonly namingMode!: GroupNamingMode;
	public readonly neededModifyPower!: number;
	public readonly neededMemberAddPower!: number;
	public readonly neededMemberRemovePower!: number | null;

	public update(obj: Partial<this>): this {
		Object.assign(this, obj);
		return this;
	}
}

export class ChannelGroup extends ChannelGroupBase {
	public readonly id!: ChannelGroupId;
	public readonly name!: string;
	public readonly groupType!: GroupType;
	public readonly icon!: IconId;
	public readonly isPermanent!: boolean;
	public readonly sortId!: number;
	public readonly namingMode!: GroupNamingMode;
	public readonly neededModifyPower!: number;
	public readonly neededMemberAddPower!: number;
	public readonly neededMemberRemovePower!: number | null;

	public update(obj: Partial<this>): this {
		Object.assign(this, obj);
		return this;
	}
}

// Setter

export interface OChangeChannelEdit {
	ChannelEdit: {
		id: ChannelId;

		password?: string | null;
		channelType?: ChannelType;
		maxClients?: MaxClients;
		maxFamilyClients?: MaxClients;
		description?: string;
		order?: ChannelId;
		name?: string;
		topic?: string;
		isDefault?: boolean;
		codec?: Codec;
		codecQuality?: number;
		neededTalkPower?: number;
		icon?: IconId;
		codecLatencyFactor?: number;
		isUnencrypted?: boolean;
		deleteDelay?: Duration;
		phoneticName?: string;
	};
}

export interface OChangeChannelDescriptionRequest {
	ChannelDescriptionRequest: {
		id: ChannelId;

	};
}

export interface OChangeChannelMove {
	ChannelMove: {
		id: ChannelId;

		parent: ChannelId,
		order: ChannelId,
	};
}

export interface OChangeClientConnectionInfoRequest {
	ClientConnectionInfoRequest: {
		id: ClientId;

	};
}

export interface OChangeClientVariablesRequest {
	ClientVariablesRequest: {
		id: ClientId;

	};
}

export interface OChangeClientEdit {
	ClientEdit: {
		id: ClientId;

		description?: string;
		talkPowerGranted?: boolean;
	};
}

export interface OChangeClientUpdate {
	ClientUpdate: {
		
		name?: string;
		inputMuted?: boolean;
		outputMuted?: boolean;
		away?: string | null;
	};
}

export interface OChangeConnectionRemove {
	ConnectionRemove: {
		
	};
}

export interface OChangeClientAddServerGroup {
	ClientAddServerGroup: {
		id: ClientId;

		serverGroup?: ServerGroupId;
	};
}

export interface OChangeClientRemoveServerGroup {
	ClientRemoveServerGroup: {
		id: ClientId;

		serverGroup?: ServerGroupId;
	};
}

export interface OChangeClientMove {
	ClientMove: {
		id: ClientId;

		channel: ChannelId,
		password?: string;
	};
}

export interface OChangeClientKick {
	ClientKick: {
		id: ClientId;

		reason: Reason,
		reasonMessage?: string;
	};
}

export interface OChangeServerVariablesRequest {
	ServerVariablesRequest: {
		
	};
}

export interface OChangeServerConnectionInfoRequest {
	ServerConnectionInfoRequest: {
		
	};
}

export interface OChangeServerGroupAddClient {
	ServerGroupAddClient: {
		id: ServerGroupId;

		client?: ClientDbId;
	};
}

export interface OChangeServerGroupRemoveClient {
	ServerGroupRemoveClient: {
		id: ServerGroupId;

		client?: ClientDbId;
	};
}

export type OChange =
	OChangeChannelEdit
	| OChangeChannelDescriptionRequest
	| OChangeChannelMove
	| OChangeClientConnectionInfoRequest
	| OChangeClientVariablesRequest
	| OChangeClientEdit
	| OChangeClientUpdate
	| OChangeConnectionRemove
	| OChangeClientAddServerGroup
	| OChangeClientRemoveServerGroup
	| OChangeClientMove
	| OChangeClientKick
	| OChangeServerVariablesRequest
	| OChangeServerConnectionInfoRequest
	| OChangeServerGroupAddClient
	| OChangeServerGroupRemoveClient
;
