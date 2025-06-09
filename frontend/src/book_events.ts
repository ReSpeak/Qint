import { ChannelGroupId, ChannelId, ClientDbId, ClientId, ClientType, EccKeyPubP256, IconId, IpAddr,
	MaxClients, OffsetDateTime, Permission, RustDuration, ServerGroupId, SocketAddr,
	TalkPowerRequest, Uid,
} from "./ts";
import { datetimeDeserialize, durationDeserialize } from "./util";
import { Duration, Moment } from "moment";
import { ServerGroupBase, ChannelGroupBase, OptionalChannelDataBase, ChannelBase, OptionalClientDataBase, ConnectionClientDataBase, ClientBase, OptionalServerDataBase, ConnectionServerDataBase, ServerBase } from "./bookBase";

// Enums

export enum PermissionType {
	ServerGroup = "ServerGroup",
	GlobalClient = "GlobalClient",
	Channel = "Channel",
	ChannelGroup = "ChannelGroup",
	ChannelClient = "ChannelClient",
}

export function permissionTypeGetDoc(v: PermissionType): string {
	switch(v) {
		case PermissionType.ServerGroup:
			return "Server group permission. (id1: ServerGroupId, id2: 0)";
		case PermissionType.GlobalClient:
			return "Client specific permission. (id1: ClientDbId, id2: 0)";
		case PermissionType.Channel:
			return "Channel specific permission. (id1: ChannelId, id2: 0)";
		case PermissionType.ChannelGroup:
			return "Channel group permission. (id1: ChannelId, id2: ChannelGroupId)";
		case PermissionType.ChannelClient:
			return "Channel-client specific permission. (id1: ChannelId, id2: ClientDbId)";
	}
}

export enum TextMessageTargetMode {
	Unknown = "Unknown",
	Client = "Client",
	Channel = "Channel",
	Server = "Server",
}

export function textMessageTargetModeGetDoc(v: TextMessageTargetMode): string {
	switch(v) {
		case TextMessageTargetMode.Unknown:
			return "Maybe to all servers?";
		case TextMessageTargetMode.Client:
			return "Send to specific client";
		case TextMessageTargetMode.Channel:
			return "Send to current channel";
		case TextMessageTargetMode.Server:
			return "Send to server chat";
	}
}

export enum HostMessageMode {
	None = "None",
	Log = "Log",
	Modal = "Modal",
	Modalquit = "Modalquit",
}

export function hostMessageModeGetDoc(v: HostMessageMode): string {
	switch(v) {
		case HostMessageMode.None:
			return "Dont display anything";
		case HostMessageMode.Log:
			return "Display message inside log";
		case HostMessageMode.Modal:
			return "Display message inside a modal dialog";
		case HostMessageMode.Modalquit:
			return "Display message inside a modal dialog and quit/close server/connection";
	}
}

export enum HostBannerMode {
	NoAdjust = "NoAdjust",
	AdjustIgnoreAspect = "AdjustIgnoreAspect",
	AdjustKeepAspect = "AdjustKeepAspect",
}

export function hostBannerModeGetDoc(v: HostBannerMode): string {
	switch(v) {
		case HostBannerMode.NoAdjust:
			return "Do not adjust";
		case HostBannerMode.AdjustIgnoreAspect:
			return "Adjust and ignore aspect ratio";
		case HostBannerMode.AdjustKeepAspect:
			return "Adjust and keep aspect ratio";
	}
}

export enum Codec {
	SpeexNarrowband = "SpeexNarrowband",
	SpeexWideband = "SpeexWideband",
	SpeexUltrawideband = "SpeexUltrawideband",
	CeltMono = "CeltMono",
	OpusVoice = "OpusVoice",
	OpusMusic = "OpusMusic",
}

export function codecGetDoc(v: Codec): string {
	switch(v) {
		case Codec.SpeexNarrowband:
			return "Mono, 16bit, 8kHz, bitrate dependent on the quality setting";
		case Codec.SpeexWideband:
			return "Mono, 16bit, 16kHz, bitrate dependent on the quality setting";
		case Codec.SpeexUltrawideband:
			return "Mono, 16bit, 32kHz, bitrate dependent on the quality setting";
		case Codec.CeltMono:
			return "Mono, 16bit, 48kHz, bitrate dependent on the quality setting";
		case Codec.OpusVoice:
			return "Mono, 16bit, 48kHz, bitrate dependent on the quality setting, optimized for voice";
		case Codec.OpusMusic:
			return "Stereo, 16bit, 48kHz, bitrate dependent on the quality setting, optimized for music";
	}
}

export enum CodecEncryptionMode {
	PerChannel = "PerChannel",
	ForcedOff = "ForcedOff",
	ForcedOn = "ForcedOn",
}

export function codecEncryptionModeGetDoc(v: CodecEncryptionMode): string {
	switch(v) {
		case CodecEncryptionMode.PerChannel:
			return "Voice encryption is configured per channel";
		case CodecEncryptionMode.ForcedOff:
			return "Voice encryption is globally off";
		case CodecEncryptionMode.ForcedOn:
			return "Voice encryption is globally on";
	}
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

export function reasonGetDoc(v: Reason): string {
	switch(v) {
		case Reason.None:
			return "No reason data";
		case Reason.Moved:
			return "Has invoker";
		case Reason.Subscription:
			return "No reason data";
		case Reason.LostConnection:
			return "Timeout";
		case Reason.KickChannel:
			return "Has invoker";
		case Reason.KickServer:
			return "Has invoker";
		case Reason.KickServerBan:
			return "Has invoker, bantime";
		case Reason.Serverstop:
			return "";
		case Reason.Clientdisconnect:
			return "";
		case Reason.Channelupdate:
			return "No reason data";
		case Reason.Channeledit:
			return "Has invoker";
		case Reason.ClientdisconnectServerShutdown:
			return "";
	}
}

export enum GroupNamingMode {
	None = "None",
	Before = "Before",
	After = "After",
}

export function groupNamingModeGetDoc(v: GroupNamingMode): string {
	switch(v) {
		case GroupNamingMode.None:
			return "No group name is displayed.";
		case GroupNamingMode.Before:
			return "Group name is displayed before the client name.";
		case GroupNamingMode.After:
			return "Group name is displayed after the client name.";
	}
}

export enum GroupType {
	Template = "Template",
	Regular = "Regular",
	Query = "Query",
}

export function groupTypeGetDoc(v: GroupType): string {
	switch(v) {
		case GroupType.Template:
			return "Template group (used for new virtual servers).";
		case GroupType.Regular:
			return "Regular group (used for regular clients).";
		case GroupType.Query:
			return "Global query group (used for server query clients).";
	}
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

export function licenseTypeGetDoc(v: LicenseType): string {
	switch(v) {
		case LicenseType.NoLicense:
			return "No licence";
		case LicenseType.Offline:
			return "Offline/LAN license";
		case LicenseType.Sdk:
			return "TeamSpeak SDK license";
		case LicenseType.SdkOffline:
			return "TeamSpeak SDK offline license";
		case LicenseType.Npl:
			return "Non-Profit License (NPL)";
		case LicenseType.Athp:
			return "Authorised TeamSpeak Host Provider License (ATHP)";
		case LicenseType.Aal:
			return "Annual activation license (AAL)";
		case LicenseType.Default:
			return "Default license with 32 slots";
		case LicenseType.Gamer:
			return "Gamer license";
		case LicenseType.Sponsorship:
			return "Licenses sponsored by TeamSpeak";
		case LicenseType.Commercial:
			return "For use inside corporates";
	}
}

export enum ChannelType {
	Permanent = "Permanent",
	SemiPermanent = "SemiPermanent",
	Temporary = "Temporary",
}

export function channelTypeGetDoc(v: ChannelType): string {
	switch(v) {
		case ChannelType.Permanent:
			return "Normal channel";
		case ChannelType.SemiPermanent:
			return "Deleted when the server restarts";
		case ChannelType.Temporary:
			return "Deleted when empty";
	}
}

export enum TokenType {
	ServerGroup = "ServerGroup",
	ChannelGroup = "ChannelGroup",
}

export function tokenTypeGetDoc(v: TokenType): string {
	switch(v) {
		case TokenType.ServerGroup:
			return "Server group token (`id1={groupId}, id2=0`)";
		case TokenType.ChannelGroup:
			return "Channel group token (`id1={groupId}, id2={channelId}`)";
	}
}

export enum PluginTargetMode {
	CurrentChannel = "CurrentChannel",
	Server = "Server",
	Client = "Client",
	CurrentChannelSubsribedClients = "CurrentChannelSubsribedClients",
}

export function pluginTargetModeGetDoc(v: PluginTargetMode): string {
	switch(v) {
		case PluginTargetMode.CurrentChannel:
			return "Send to all clients in the current channel.";
		case PluginTargetMode.Server:
			return "Send to all clients on the server.";
		case PluginTargetMode.Client:
			return "Send to all given clients ids.";
		case PluginTargetMode.CurrentChannelSubsribedClients:
			return "Send to all given clients which are subscribed to the current channel (i.e. which see the this client).";
	}
}

export enum LogLevel {
	Error = "Error",
	Warning = "Warning",
	Debug = "Debug",
	Info = "Info",
}

export function logLevelGetDoc(v: LogLevel): string {
	switch(v) {
		case LogLevel.Error:
			return "Everything that is really bad.";
		case LogLevel.Warning:
			return "Everything that might be bad.";
		case LogLevel.Debug:
			return "Output that might help find a problem.";
		case LogLevel.Info:
			return "Informational output.";
	}
}

export const enum TsError {
	Ok = "Ok",
	Undefined = "Undefined",
	NotImplemented = "NotImplemented",
	OkNoUpdate = "OkNoUpdate",
	DontNotify = "DontNotify",
	LibTimeLimitReached = "LibTimeLimitReached",
	CommandNotFound = "CommandNotFound",
	UnableToBindNetworkPort = "UnableToBindNetworkPort",
	NoNetworkPortAvailable = "NoNetworkPortAvailable",
	ClientInvalidId = "ClientInvalidId",
	ClientNicknameInuse = "ClientNicknameInuse",
	ClientInvalidErrorCode = "ClientInvalidErrorCode",
	ClientProtocolLimitReached = "ClientProtocolLimitReached",
	ClientInvalidType = "ClientInvalidType",
	ClientAlreadySubscribed = "ClientAlreadySubscribed",
	ClientNotLoggedIn = "ClientNotLoggedIn",
	ClientCouldNotValidateIdentity = "ClientCouldNotValidateIdentity",
	ClientInvalidPassword = "ClientInvalidPassword",
	ClientTooManyClonesConnected = "ClientTooManyClonesConnected",
	ClientVersionOutdated = "ClientVersionOutdated",
	ClientIsOnline = "ClientIsOnline",
	ClientIsFlooding = "ClientIsFlooding",
	ClientHacked = "ClientHacked",
	ClientCannotVerifyNow = "ClientCannotVerifyNow",
	ClientLoginNotPermitted = "ClientLoginNotPermitted",
	ClientNotSubscribed = "ClientNotSubscribed",
	ChannelInvalidId = "ChannelInvalidId",
	ChannelProtocolLimitReached = "ChannelProtocolLimitReached",
	ChannelAlreadyIn = "ChannelAlreadyIn",
	ChannelNameInuse = "ChannelNameInuse",
	ChannelNotEmpty = "ChannelNotEmpty",
	ChannelCanNotDeleteDefault = "ChannelCanNotDeleteDefault",
	ChannelDefaultRequirePermanent = "ChannelDefaultRequirePermanent",
	ChannelInvalidFlags = "ChannelInvalidFlags",
	ChannelParentNotPermanent = "ChannelParentNotPermanent",
	ChannelMaxclientsReached = "ChannelMaxclientsReached",
	ChannelMaxfamilyReached = "ChannelMaxfamilyReached",
	ChannelInvalidOrder = "ChannelInvalidOrder",
	ChannelNoFiletransferSupported = "ChannelNoFiletransferSupported",
	ChannelInvalidPassword = "ChannelInvalidPassword",
	ChannelIsPrivateChannel = "ChannelIsPrivateChannel",
	ChannelInvalidSecurityHash = "ChannelInvalidSecurityHash",
	ServerInvalidId = "ServerInvalidId",
	ServerRunning = "ServerRunning",
	ServerIsShuttingDown = "ServerIsShuttingDown",
	ServerMaxclientsReached = "ServerMaxclientsReached",
	ServerInvalidPassword = "ServerInvalidPassword",
	ServerDeploymentActive = "ServerDeploymentActive",
	ServerUnableToStopOwnServer = "ServerUnableToStopOwnServer",
	ServerIsVirtual = "ServerIsVirtual",
	ServerWrongMachineid = "ServerWrongMachineid",
	ServerIsNotRunning = "ServerIsNotRunning",
	ServerIsBooting = "ServerIsBooting",
	ServerStatusInvalid = "ServerStatusInvalid",
	ServerModalQuit = "ServerModalQuit",
	ServerVersionOutdated = "ServerVersionOutdated",
	Database = "Database",
	DatabaseEmptyResult = "DatabaseEmptyResult",
	DatabaseDuplicateEntry = "DatabaseDuplicateEntry",
	DatabaseNoModifications = "DatabaseNoModifications",
	DatabaseConstraint = "DatabaseConstraint",
	DatabaseReinvoke = "DatabaseReinvoke",
	ParameterQuote = "ParameterQuote",
	ParameterInvalidCount = "ParameterInvalidCount",
	ParameterInvalid = "ParameterInvalid",
	ParameterNotFound = "ParameterNotFound",
	ParameterConvert = "ParameterConvert",
	ParameterInvalidSize = "ParameterInvalidSize",
	ParameterMissing = "ParameterMissing",
	ParameterChecksum = "ParameterChecksum",
	VsCritical = "VsCritical",
	ConnectionLost = "ConnectionLost",
	NotConnected = "NotConnected",
	NoCachedConnectionInfo = "NoCachedConnectionInfo",
	CurrentlyNotPossible = "CurrentlyNotPossible",
	FailedConnectionInitialisation = "FailedConnectionInitialisation",
	CouldNotResolveHostname = "CouldNotResolveHostname",
	InvalidServerConnectionHandlerId = "InvalidServerConnectionHandlerId",
	CouldNotInitialiseInputManager = "CouldNotInitialiseInputManager",
	ClientlibraryNotInitialised = "ClientlibraryNotInitialised",
	ServerlibraryNotInitialised = "ServerlibraryNotInitialised",
	WhisperTooManyTargets = "WhisperTooManyTargets",
	WhisperNoTargets = "WhisperNoTargets",
	FileInvalidName = "FileInvalidName",
	FileInvalidPermissions = "FileInvalidPermissions",
	FileAlreadyExists = "FileAlreadyExists",
	FileNotFound = "FileNotFound",
	FileIoError = "FileIoError",
	FileInvalidTransferId = "FileInvalidTransferId",
	FileInvalidPath = "FileInvalidPath",
	FileNoFilesAvailable = "FileNoFilesAvailable",
	FileOverwriteExcludesResume = "FileOverwriteExcludesResume",
	FileInvalidSize = "FileInvalidSize",
	FileAlreadyInUse = "FileAlreadyInUse",
	FileCouldNotOpenConnection = "FileCouldNotOpenConnection",
	FileNoSpaceLeftOnDevice = "FileNoSpaceLeftOnDevice",
	FileExceedsFileSystemMaximumSize = "FileExceedsFileSystemMaximumSize",
	FileTransferConnectionTimeout = "FileTransferConnectionTimeout",
	FileConnectionLost = "FileConnectionLost",
	FileExceedsSuppliedSize = "FileExceedsSuppliedSize",
	FileTransferComplete = "FileTransferComplete",
	FileTransferCanceled = "FileTransferCanceled",
	FileTransferInterrupted = "FileTransferInterrupted",
	FileTransferServerQuotaExceeded = "FileTransferServerQuotaExceeded",
	FileTransferClientQuotaExceeded = "FileTransferClientQuotaExceeded",
	FileTransferReset = "FileTransferReset",
	FileTransferLimitReached = "FileTransferLimitReached",
	SoundPreprocessorDisabled = "SoundPreprocessorDisabled",
	SoundInternalPreprocessor = "SoundInternalPreprocessor",
	SoundInternalEncoder = "SoundInternalEncoder",
	SoundInternalPlayback = "SoundInternalPlayback",
	SoundNoCaptureDeviceAvailable = "SoundNoCaptureDeviceAvailable",
	SoundNoPlaybackDeviceAvailable = "SoundNoPlaybackDeviceAvailable",
	SoundCouldNotOpenCaptureDevice = "SoundCouldNotOpenCaptureDevice",
	SoundCouldNotOpenPlaybackDevice = "SoundCouldNotOpenPlaybackDevice",
	SoundHandlerHasDevice = "SoundHandlerHasDevice",
	SoundInvalidCaptureDevice = "SoundInvalidCaptureDevice",
	SoundInvalidPlaybackDevice = "SoundInvalidPlaybackDevice",
	SoundInvalidWave = "SoundInvalidWave",
	SoundUnsupportedWave = "SoundUnsupportedWave",
	SoundOpenWave = "SoundOpenWave",
	SoundInternalCapture = "SoundInternalCapture",
	SoundDeviceInUse = "SoundDeviceInUse",
	SoundDeviceAlreadyRegisterred = "SoundDeviceAlreadyRegisterred",
	SoundUnknownDevice = "SoundUnknownDevice",
	SoundUnsupportedFrequency = "SoundUnsupportedFrequency",
	SoundInvalidChannelCount = "SoundInvalidChannelCount",
	SoundReadWave = "SoundReadWave",
	SoundNeedMoreData = "SoundNeedMoreData",
	SoundDeviceBusy = "SoundDeviceBusy",
	SoundNoData = "SoundNoData",
	SoundChannelMaskMismatch = "SoundChannelMaskMismatch",
	PermissionInvalidGroupId = "PermissionInvalidGroupId",
	PermissionDuplicateEntry = "PermissionDuplicateEntry",
	PermissionInvalidPermId = "PermissionInvalidPermId",
	PermissionEmptyResult = "PermissionEmptyResult",
	PermissionDefaultGroupForbidden = "PermissionDefaultGroupForbidden",
	PermissionInvalidSize = "PermissionInvalidSize",
	PermissionInvalidValue = "PermissionInvalidValue",
	PermissionsGroupNotEmpty = "PermissionsGroupNotEmpty",
	PermissionsClientInsufficient = "PermissionsClientInsufficient",
	PermissionsInsufficientGroupPower = "PermissionsInsufficientGroupPower",
	PermissionsInsufficientPermissionPower = "PermissionsInsufficientPermissionPower",
	PermissionTemplateGroupIsUsed = "PermissionTemplateGroupIsUsed",
	Permissions = "Permissions",
	AccountingVirtualserverLimitReached = "AccountingVirtualserverLimitReached",
	AccountingSlotLimitReached = "AccountingSlotLimitReached",
	AccountingLicenseFileNotFound = "AccountingLicenseFileNotFound",
	AccountingLicenseDateNotOk = "AccountingLicenseDateNotOk",
	AccountingUnableToConnectToServer = "AccountingUnableToConnectToServer",
	AccountingUnknownError = "AccountingUnknownError",
	AccountingServerError = "AccountingServerError",
	AccountingInstanceLimitReached = "AccountingInstanceLimitReached",
	AccountingInstanceCheckError = "AccountingInstanceCheckError",
	AccountingLicenseFileInvalid = "AccountingLicenseFileInvalid",
	AccountingRunningElsewhere = "AccountingRunningElsewhere",
	AccountingInstanceDuplicated = "AccountingInstanceDuplicated",
	AccountingAlreadyStarted = "AccountingAlreadyStarted",
	AccountingNotStarted = "AccountingNotStarted",
	AccountingToManyStarts = "AccountingToManyStarts",
	MessageInvalidId = "MessageInvalidId",
	BanInvalidId = "BanInvalidId",
	ConnectFailedBanned = "ConnectFailedBanned",
	RenameFailedBanned = "RenameFailedBanned",
	BanFlooding = "BanFlooding",
	TtsUnableToInitialize = "TtsUnableToInitialize",
	PrivilegeKeyInvalid = "PrivilegeKeyInvalid",
	VoipPjsua = "VoipPjsua",
	VoipAlreadyInitialized = "VoipAlreadyInitialized",
	VoipTooManyAccounts = "VoipTooManyAccounts",
	VoipInvalidAccount = "VoipInvalidAccount",
	VoipInternalError = "VoipInternalError",
	VoipInvalidConnectionId = "VoipInvalidConnectionId",
	VoipCannotAnswerInitiatedCall = "VoipCannotAnswerInitiatedCall",
	VoipNotInitialized = "VoipNotInitialized",
	ProvisioningInvalidPassword = "ProvisioningInvalidPassword",
	ProvisioningInvalidRequest = "ProvisioningInvalidRequest",
	ProvisioningNoSlotsAvailable = "ProvisioningNoSlotsAvailable",
	ProvisioningPoolMissing = "ProvisioningPoolMissing",
	ProvisioningPoolUnknown = "ProvisioningPoolUnknown",
	ProvisioningUnknownIpLocation = "ProvisioningUnknownIpLocation",
	ProvisioningInternalTriesExceeded = "ProvisioningInternalTriesExceeded",
	ProvisioningTooManySlotsRequested = "ProvisioningTooManySlotsRequested",
	ProvisioningTooManyReserved = "ProvisioningTooManyReserved",
	ProvisioningCouldNotConnect = "ProvisioningCouldNotConnect",
	ProvisioningAuthServerNotConnected = "ProvisioningAuthServerNotConnected",
	ProvisioningAuthDataTooLarge = "ProvisioningAuthDataTooLarge",
	ProvisioningAlreadyInitialized = "ProvisioningAlreadyInitialized",
	ProvisioningNotInitialized = "ProvisioningNotInitialized",
	ProvisioningConnecting = "ProvisioningConnecting",
	ProvisioningAlreadyConnected = "ProvisioningAlreadyConnected",
	ProvisioningNotConnected = "ProvisioningNotConnected",
	ProvisioningIoError = "ProvisioningIoError",
	ProvisioningInvalidTimeout = "ProvisioningInvalidTimeout",
	ProvisioningTs3serverNotFound = "ProvisioningTs3serverNotFound",
	ProvisioningNoPermission = "ProvisioningNoPermission",
}

export function TsErrorDescription(error: TsError): string {
	switch (error) {
		case TsError.Ok: return "unknown error code";
		case TsError.Undefined: return "undefined error";
		case TsError.NotImplemented: return "not implemented";
		case TsError.OkNoUpdate: return "";
		case TsError.DontNotify: return "";
		case TsError.LibTimeLimitReached: return "library time limit reached";
		case TsError.CommandNotFound: return "command not found";
		case TsError.UnableToBindNetworkPort: return "unable to bind network port";
		case TsError.NoNetworkPortAvailable: return "no network port available";
		case TsError.ClientInvalidId: return "invalid clientID";
		case TsError.ClientNicknameInuse: return "nickname is already in use";
		case TsError.ClientInvalidErrorCode: return "invalid error code";
		case TsError.ClientProtocolLimitReached: return "max clients protocol limit reached";
		case TsError.ClientInvalidType: return "invalid client type";
		case TsError.ClientAlreadySubscribed: return "already subscribed";
		case TsError.ClientNotLoggedIn: return "not logged in";
		case TsError.ClientCouldNotValidateIdentity: return "could not validate client identity";
		case TsError.ClientInvalidPassword: return "invalid loginname or password";
		case TsError.ClientTooManyClonesConnected: return "too many clones already connected";
		case TsError.ClientVersionOutdated: return "client version outdated, please update";
		case TsError.ClientIsOnline: return "client is online";
		case TsError.ClientIsFlooding: return "client is flooding";
		case TsError.ClientHacked: return "client is modified";
		case TsError.ClientCannotVerifyNow: return "can not verify client at this moment";
		case TsError.ClientLoginNotPermitted: return "client is not permitted to log in";
		case TsError.ClientNotSubscribed: return "client is not subscribed to the channel";
		case TsError.ChannelInvalidId: return "invalid channelID";
		case TsError.ChannelProtocolLimitReached: return "max channels protocol limit reached";
		case TsError.ChannelAlreadyIn: return "already member of channel";
		case TsError.ChannelNameInuse: return "channel name is already in use";
		case TsError.ChannelNotEmpty: return "channel not empty";
		case TsError.ChannelCanNotDeleteDefault: return "can not delete default channel";
		case TsError.ChannelDefaultRequirePermanent: return "default channel requires permanent";
		case TsError.ChannelInvalidFlags: return "invalid channel flags";
		case TsError.ChannelParentNotPermanent: return "permanent channel can not be child of non permanent channel";
		case TsError.ChannelMaxclientsReached: return "channel maxclient reached";
		case TsError.ChannelMaxfamilyReached: return "channel maxfamily reached";
		case TsError.ChannelInvalidOrder: return "invalid channel order";
		case TsError.ChannelNoFiletransferSupported: return "channel does not support filetransfers";
		case TsError.ChannelInvalidPassword: return "invalid channel password";
		case TsError.ChannelIsPrivateChannel: return "channel is private channel";
		case TsError.ChannelInvalidSecurityHash: return "invalid security hash supplied by client";
		case TsError.ServerInvalidId: return "invalid serverID";
		case TsError.ServerRunning: return "server is running";
		case TsError.ServerIsShuttingDown: return "server is shutting down";
		case TsError.ServerMaxclientsReached: return "server maxclient reached";
		case TsError.ServerInvalidPassword: return "invalid server password";
		case TsError.ServerDeploymentActive: return "deployment active";
		case TsError.ServerUnableToStopOwnServer: return "unable to stop own server in your connection class";
		case TsError.ServerIsVirtual: return "server is virtual";
		case TsError.ServerWrongMachineid: return "server wrong machineID";
		case TsError.ServerIsNotRunning: return "server is not running";
		case TsError.ServerIsBooting: return "server is booting up";
		case TsError.ServerStatusInvalid: return "server got an invalid status for this operation";
		case TsError.ServerModalQuit: return "server modal quit";
		case TsError.ServerVersionOutdated: return "server version is too old for command";
		case TsError.Database: return "database error";
		case TsError.DatabaseEmptyResult: return "database empty result set";
		case TsError.DatabaseDuplicateEntry: return "database duplicate entry";
		case TsError.DatabaseNoModifications: return "database no modifications";
		case TsError.DatabaseConstraint: return "database invalid constraint";
		case TsError.DatabaseReinvoke: return "database reinvoke command";
		case TsError.ParameterQuote: return "invalid quote";
		case TsError.ParameterInvalidCount: return "invalid parameter count";
		case TsError.ParameterInvalid: return "invalid parameter";
		case TsError.ParameterNotFound: return "parameter not found";
		case TsError.ParameterConvert: return "convert error";
		case TsError.ParameterInvalidSize: return "invalid parameter size";
		case TsError.ParameterMissing: return "missing required parameter";
		case TsError.ParameterChecksum: return "invalid checksum";
		case TsError.VsCritical: return "virtual server got a critical error";
		case TsError.ConnectionLost: return "Connection lost";
		case TsError.NotConnected: return "not connected";
		case TsError.NoCachedConnectionInfo: return "no cached connection info";
		case TsError.CurrentlyNotPossible: return "currently not possible";
		case TsError.FailedConnectionInitialisation: return "failed connection initialization";
		case TsError.CouldNotResolveHostname: return "could not resolve hostname";
		case TsError.InvalidServerConnectionHandlerId: return "invalid server connection handler ID";
		case TsError.CouldNotInitialiseInputManager: return "could not initialize Input Manager";
		case TsError.ClientlibraryNotInitialised: return "client library not initialized";
		case TsError.ServerlibraryNotInitialised: return "server library not initialized";
		case TsError.WhisperTooManyTargets: return "too many whisper targets";
		case TsError.WhisperNoTargets: return "no whisper targets found";
		case TsError.FileInvalidName: return "invalid file name";
		case TsError.FileInvalidPermissions: return "invalid file permissions";
		case TsError.FileAlreadyExists: return "file already exists";
		case TsError.FileNotFound: return "file not found";
		case TsError.FileIoError: return "file input/output error";
		case TsError.FileInvalidTransferId: return "invalid file transfer ID";
		case TsError.FileInvalidPath: return "invalid file path";
		case TsError.FileNoFilesAvailable: return "no files available";
		case TsError.FileOverwriteExcludesResume: return "overwrite excludes resume";
		case TsError.FileInvalidSize: return "invalid file size";
		case TsError.FileAlreadyInUse: return "file already in use";
		case TsError.FileCouldNotOpenConnection: return "could not open file transfer connection";
		case TsError.FileNoSpaceLeftOnDevice: return "no space left on device (disk full?)";
		case TsError.FileExceedsFileSystemMaximumSize: return "file exceeds file system's maximum file size";
		case TsError.FileTransferConnectionTimeout: return "file transfer connection timeout";
		case TsError.FileConnectionLost: return "lost file transfer connection";
		case TsError.FileExceedsSuppliedSize: return "file exceeds supplied file size";
		case TsError.FileTransferComplete: return "file transfer complete";
		case TsError.FileTransferCanceled: return "file transfer canceled";
		case TsError.FileTransferInterrupted: return "file transfer interrupted";
		case TsError.FileTransferServerQuotaExceeded: return "file transfer server quota exceeded";
		case TsError.FileTransferClientQuotaExceeded: return "file transfer client quota exceeded";
		case TsError.FileTransferReset: return "file transfer reset";
		case TsError.FileTransferLimitReached: return "file transfer limit reached";
		case TsError.SoundPreprocessorDisabled: return "preprocessor disabled";
		case TsError.SoundInternalPreprocessor: return "internal preprocessor";
		case TsError.SoundInternalEncoder: return "internal encoder";
		case TsError.SoundInternalPlayback: return "internal playback";
		case TsError.SoundNoCaptureDeviceAvailable: return "no capture device available";
		case TsError.SoundNoPlaybackDeviceAvailable: return "no playback device available";
		case TsError.SoundCouldNotOpenCaptureDevice: return "could not open capture device";
		case TsError.SoundCouldNotOpenPlaybackDevice: return "could not open playback device";
		case TsError.SoundHandlerHasDevice: return "ServerConnectionHandler has a device registered";
		case TsError.SoundInvalidCaptureDevice: return "invalid capture device";
		case TsError.SoundInvalidPlaybackDevice: return "invalid clayback device";
		case TsError.SoundInvalidWave: return "invalid wave file";
		case TsError.SoundUnsupportedWave: return "wave file type not supported";
		case TsError.SoundOpenWave: return "could not open wave file";
		case TsError.SoundInternalCapture: return "internal capture";
		case TsError.SoundDeviceInUse: return "device still in use";
		case TsError.SoundDeviceAlreadyRegisterred: return "device already registerred";
		case TsError.SoundUnknownDevice: return "device not registered/known";
		case TsError.SoundUnsupportedFrequency: return "unsupported frequency";
		case TsError.SoundInvalidChannelCount: return "invalid channel count";
		case TsError.SoundReadWave: return "read error in wave";
		case TsError.SoundNeedMoreData: return "sound need more data";
		case TsError.SoundDeviceBusy: return "sound device was busy";
		case TsError.SoundNoData: return "there is no sound data for this period";
		case TsError.SoundChannelMaskMismatch: return "Channelmask set bits count (speakers) is not the same as (count)";
		case TsError.PermissionInvalidGroupId: return "invalid group ID";
		case TsError.PermissionDuplicateEntry: return "duplicate entry";
		case TsError.PermissionInvalidPermId: return "invalid permission ID";
		case TsError.PermissionEmptyResult: return "empty result set";
		case TsError.PermissionDefaultGroupForbidden: return "access to default group is forbidden";
		case TsError.PermissionInvalidSize: return "invalid size";
		case TsError.PermissionInvalidValue: return "invalid value";
		case TsError.PermissionsGroupNotEmpty: return "group is not empty";
		case TsError.PermissionsClientInsufficient: return "insufficient client permissions";
		case TsError.PermissionsInsufficientGroupPower: return "insufficient group modify power";
		case TsError.PermissionsInsufficientPermissionPower: return "insufficient permission modify power";
		case TsError.PermissionTemplateGroupIsUsed: return "template group is currently used";
		case TsError.Permissions: return "permission error";
		case TsError.AccountingVirtualserverLimitReached: return "virtualserver limit reached";
		case TsError.AccountingSlotLimitReached: return "max slot limit reached";
		case TsError.AccountingLicenseFileNotFound: return "license file not found";
		case TsError.AccountingLicenseDateNotOk: return "license date not ok";
		case TsError.AccountingUnableToConnectToServer: return "unable to connect to accounting server";
		case TsError.AccountingUnknownError: return "unknown accounting error";
		case TsError.AccountingServerError: return "accounting server error";
		case TsError.AccountingInstanceLimitReached: return "instance limit reached";
		case TsError.AccountingInstanceCheckError: return "instance check error";
		case TsError.AccountingLicenseFileInvalid: return "license file invalid";
		case TsError.AccountingRunningElsewhere: return "virtualserver is running elsewhere";
		case TsError.AccountingInstanceDuplicated: return "virtualserver running in same instance already";
		case TsError.AccountingAlreadyStarted: return "virtualserver already started";
		case TsError.AccountingNotStarted: return "virtualserver not started";
		case TsError.AccountingToManyStarts: return "";
		case TsError.MessageInvalidId: return "invalid message id";
		case TsError.BanInvalidId: return "invalid ban id";
		case TsError.ConnectFailedBanned: return "connection failed, you are banned";
		case TsError.RenameFailedBanned: return "rename failed, new name is banned";
		case TsError.BanFlooding: return "flood ban";
		case TsError.TtsUnableToInitialize: return "unable to initialize tts";
		case TsError.PrivilegeKeyInvalid: return "invalid privilege key";
		case TsError.VoipPjsua: return "";
		case TsError.VoipAlreadyInitialized: return "";
		case TsError.VoipTooManyAccounts: return "";
		case TsError.VoipInvalidAccount: return "";
		case TsError.VoipInternalError: return "";
		case TsError.VoipInvalidConnectionId: return "";
		case TsError.VoipCannotAnswerInitiatedCall: return "";
		case TsError.VoipNotInitialized: return "";
		case TsError.ProvisioningInvalidPassword: return "invalid password";
		case TsError.ProvisioningInvalidRequest: return "invalid request";
		case TsError.ProvisioningNoSlotsAvailable: return "no(more) slots available";
		case TsError.ProvisioningPoolMissing: return "pool missing";
		case TsError.ProvisioningPoolUnknown: return "pool unknown";
		case TsError.ProvisioningUnknownIpLocation: return "unknown ip location(perhaps LAN ip?)";
		case TsError.ProvisioningInternalTriesExceeded: return "internal error(tried exceeded)";
		case TsError.ProvisioningTooManySlotsRequested: return "too many slots requested";
		case TsError.ProvisioningTooManyReserved: return "too many reserved";
		case TsError.ProvisioningCouldNotConnect: return "could not connect to provisioning server";
		case TsError.ProvisioningAuthServerNotConnected: return "authentication server not connected";
		case TsError.ProvisioningAuthDataTooLarge: return "authentication data too large";
		case TsError.ProvisioningAlreadyInitialized: return "already initialized";
		case TsError.ProvisioningNotInitialized: return "not initialized";
		case TsError.ProvisioningConnecting: return "already connecting";
		case TsError.ProvisioningAlreadyConnected: return "already connected";
		case TsError.ProvisioningNotConnected: return "";
		case TsError.ProvisioningIoError: return "io_error";
		case TsError.ProvisioningInvalidTimeout: return "";
		case TsError.ProvisioningTs3serverNotFound: return "";
		case TsError.ProvisioningNoPermission: return "unknown permissionID";
	}
}

export const enum Version {
	Windows_3_0_11__1 = "Windows_3_0_11__1",
	Windows_3_0_11__2 = "Windows_3_0_11__2",
	Windows_3_0_11__3 = "Windows_3_0_11__3",
	Windows_3_0_11_1__1 = "Windows_3_0_11_1__1",
	Linux_3_0_11_1 = "Linux_3_0_11_1",
	OS_X_3_0_11_1 = "OS_X_3_0_11_1",
	Windows_3_0_11_1__2 = "Windows_3_0_11_1__2",
	Windows_3_0_12__1 = "Windows_3_0_12__1",
	Windows_3_0_12__2 = "Windows_3_0_12__2",
	Windows_3_0_12__3 = "Windows_3_0_12__3",
	Windows_3_0_12__4 = "Windows_3_0_12__4",
	Windows_3_0_13__1 = "Windows_3_0_13__1",
	Windows_3_0_13__2 = "Windows_3_0_13__2",
	Android_3_0_13 = "Android_3_0_13",
	OS_X_3_0_13_1 = "OS_X_3_0_13_1",
	Windows_3_0_13_1 = "Windows_3_0_13_1",
	Windows_3_0_14__1 = "Windows_3_0_14__1",
	Windows_3_0_14__2 = "Windows_3_0_14__2",
	Windows_3_0_14__3 = "Windows_3_0_14__3",
	Windows_3_0_14__4 = "Windows_3_0_14__4",
	Windows_3_0_14__5 = "Windows_3_0_14__5",
	Windows_3_0_15__1 = "Windows_3_0_15__1",
	Windows_3_0_15__2 = "Windows_3_0_15__2",
	Windows_3_0_15__3 = "Windows_3_0_15__3",
	Windows_3_0_15_1 = "Windows_3_0_15_1",
	Windows_3_0_16__1 = "Windows_3_0_16__1",
	Linux_3_0_16 = "Linux_3_0_16",
	OS_X_3_0_16 = "OS_X_3_0_16",
	Windows_3_0_16__2 = "Windows_3_0_16__2",
	Android_3_0_19 = "Android_3_0_19",
	Windows_3_0_17__1 = "Windows_3_0_17__1",
	Windows_3_0_17__2 = "Windows_3_0_17__2",
	Windows_3_0_17__3 = "Windows_3_0_17__3",
	Windows_3_0_17__4 = "Windows_3_0_17__4",
	Windows_3_0_17__5 = "Windows_3_0_17__5",
	Windows_3_0_18__1 = "Windows_3_0_18__1",
	Windows_3_0_18__2 = "Windows_3_0_18__2",
	Windows_3_0_18__3 = "Windows_3_0_18__3",
	Windows_3_0_18__4 = "Windows_3_0_18__4",
	Windows_3_0_18_1 = "Windows_3_0_18_1",
	Windows_3_0_19__1 = "Windows_3_0_19__1",
	Linux_3_0_18_2 = "Linux_3_0_18_2",
	OS_X_3_0_18_2 = "OS_X_3_0_18_2",
	Windows_3_0_18_2 = "Windows_3_0_18_2",
	iOS_3_0_18_2 = "iOS_3_0_18_2",
	Android_3_0_20_2 = "Android_3_0_20_2",
	Android_3_0_21 = "Android_3_0_21",
	Windows_3_0_19__2 = "Windows_3_0_19__2",
	Windows_3_0_19__3 = "Windows_3_0_19__3",
	Linux_3_0_19 = "Linux_3_0_19",
	Windows_3_0_19__4 = "Windows_3_0_19__4",
	OS_X_3_0_19_1 = "OS_X_3_0_19_1",
	Windows_3_0_19_1 = "Windows_3_0_19_1",
	Android_3_0_23 = "Android_3_0_23",
	Windows_3_0_20 = "Windows_3_0_20",
	Windows_3_0_19_2 = "Windows_3_0_19_2",
	Windows_3_0_19_3 = "Windows_3_0_19_3",
	Linux_3_0_19_4 = "Linux_3_0_19_4",
	OS_X_3_0_19_4 = "OS_X_3_0_19_4",
	Windows_3_0_19_4 = "Windows_3_0_19_4",
	Windows_3_1__1 = "Windows_3_1__1",
	Windows_3_1__2 = "Windows_3_1__2",
	Linux_3_1__1 = "Linux_3_1__1",
	Windows_3_1__3 = "Windows_3_1__3",
	Linux_3_1__2 = "Linux_3_1__2",
	Windows_3_1__4 = "Windows_3_1__4",
	Linux_3_1__3 = "Linux_3_1__3",
	Windows_3_1__5 = "Windows_3_1__5",
	Linux_3_1__4 = "Linux_3_1__4",
	Windows_3_1__6 = "Windows_3_1__6",
	Linux_3_1__5 = "Linux_3_1__5",
	Windows_3_1__7 = "Windows_3_1__7",
	iOS_3_1 = "iOS_3_1",
	Linux_3_1__6 = "Linux_3_1__6",
	Windows_3_1__8 = "Windows_3_1__8",
	Android_3_1_0 = "Android_3_1_0",
	Linux_3_1_0_1 = "Linux_3_1_0_1",
	Windows_3_1_0_1 = "Windows_3_1_0_1",
	Linux_3_1_1__1 = "Linux_3_1_1__1",
	Windows_3_1_1__1 = "Windows_3_1_1__1",
	Linux_3_1_1__2 = "Linux_3_1_1__2",
	Windows_3_1_1__2 = "Windows_3_1_1__2",
	Linux_3_1_1__3 = "Linux_3_1_1__3",
	Windows_3_1_1__3 = "Windows_3_1_1__3",
	Linux_3_1_1_1 = "Linux_3_1_1_1",
	OS_X_3_1_1_1 = "OS_X_3_1_1_1",
	Windows_3_1_1_1 = "Windows_3_1_1_1",
	Android_3_1_0_2 = "Android_3_1_0_2",
	Linux_3_1_2 = "Linux_3_1_2",
	OS_X_3_1_2 = "OS_X_3_1_2",
	Windows_3_1_2 = "Windows_3_1_2",
	Linux_3_1_3 = "Linux_3_1_3",
	Windows_3_1_3 = "Windows_3_1_3",
	Android_3_1_2 = "Android_3_1_2",
	iOS_3_1_2 = "iOS_3_1_2",
	Linux_3_1_4 = "Linux_3_1_4",
	OS_X_3_1_4 = "OS_X_3_1_4",
	Windows_3_1_4 = "Windows_3_1_4",
	Android_3_1_3_1 = "Android_3_1_3_1",
	Linux_3_1_5__1 = "Linux_3_1_5__1",
	Windows_3_1_5__1 = "Windows_3_1_5__1",
	Linux_3_1_5__2 = "Linux_3_1_5__2",
	Windows_3_1_5__2 = "Windows_3_1_5__2",
	Linux_3_1_4_2 = "Linux_3_1_4_2",
	OS_X_3_1_4_2 = "OS_X_3_1_4_2",
	Windows_3_1_4_2 = "Windows_3_1_4_2",
	Linux_3_1_5__3 = "Linux_3_1_5__3",
	Windows_3_1_5__3 = "Windows_3_1_5__3",
	Linux_3_1_5__4 = "Linux_3_1_5__4",
	Windows_3_1_5__4 = "Windows_3_1_5__4",
	Linux_3_1_5__5 = "Linux_3_1_5__5",
	Windows_3_1_5__5 = "Windows_3_1_5__5",
	Linux_3_1_5__6 = "Linux_3_1_5__6",
	OS_X_3_1_5 = "OS_X_3_1_5",
	Windows_3_1_5__6 = "Windows_3_1_5__6",
	Linux_3_1_6__1 = "Linux_3_1_6__1",
	Windows_3_1_6__1 = "Windows_3_1_6__1",
	Android_3_1_6 = "Android_3_1_6",
	Linux_3_1_6__2 = "Linux_3_1_6__2",
	OS_X_3_1_6 = "OS_X_3_1_6",
	Windows_3_1_6__2 = "Windows_3_1_6__2",
	iOS_3_1_6 = "iOS_3_1_6",
	OS_X_3_1_7__1 = "OS_X_3_1_7__1",
	Windows_3_1_7__1 = "Windows_3_1_7__1",
	OS_X_3_1_7__2 = "OS_X_3_1_7__2",
	OS_X_3_1_7__3 = "OS_X_3_1_7__3",
	OS_X_3_1_7__4 = "OS_X_3_1_7__4",
	Windows_3_1_7__2 = "Windows_3_1_7__2",
	Android_3_1_7 = "Android_3_1_7",
	Linux_3_1_7 = "Linux_3_1_7",
	OS_X_3_1_7__5 = "OS_X_3_1_7__5",
	Windows_3_1_7__3 = "Windows_3_1_7__3",
	Windows_3_1_8__1 = "Windows_3_1_8__1",
	Linux_3_1_8__1 = "Linux_3_1_8__1",
	OS_X_3_1_8__1 = "OS_X_3_1_8__1",
	Windows_3_1_8__2 = "Windows_3_1_8__2",
	Linux_3_1_8__2 = "Linux_3_1_8__2",
	OS_X_3_1_8__2 = "OS_X_3_1_8__2",
	Windows_3_1_8__3 = "Windows_3_1_8__3",
	Android_3_1_8__1 = "Android_3_1_8__1",
	Android_3_1_8__2 = "Android_3_1_8__2",
	iOS_3_1_8 = "iOS_3_1_8",
	OS_X_3_1_9__1 = "OS_X_3_1_9__1",
	OS_X_3_1_9__2 = "OS_X_3_1_9__2",
	OS_X_3_1_9__3 = "OS_X_3_1_9__3",
	OS_X_3_1_9__4 = "OS_X_3_1_9__4",
	Linux_3_1_9 = "Linux_3_1_9",
	OS_X_3_1_9__5 = "OS_X_3_1_9__5",
	Windows_3_1_9 = "Windows_3_1_9",
	Linux_3_1_10 = "Linux_3_1_10",
	OS_X_3_1_10 = "OS_X_3_1_10",
	Windows_3_1_10 = "Windows_3_1_10",
	Windows_3_2_0__1 = "Windows_3_2_0__1",
	Windows_3_2_0__2 = "Windows_3_2_0__2",
	Windows_3_2_0__3 = "Windows_3_2_0__3",
	Android_3_2_0__1 = "Android_3_2_0__1",
	Windows_3_2_0__4 = "Windows_3_2_0__4",
	OS_X_3_2_0__1 = "OS_X_3_2_0__1",
	Windows_3_2_0__5 = "Windows_3_2_0__5",
	Windows_3_2_0__6 = "Windows_3_2_0__6",
	Windows_3_2_0__7 = "Windows_3_2_0__7",
	Linux_3_2_0 = "Linux_3_2_0",
	OS_X_3_2_0__2 = "OS_X_3_2_0__2",
	Windows_3_2_0__8 = "Windows_3_2_0__8",
	Linux_3_2_1 = "Linux_3_2_1",
	OS_X_3_2_1 = "OS_X_3_2_1",
	Windows_3_2_1 = "Windows_3_2_1",
	Android_3_2_0__2 = "Android_3_2_0__2",
	iOS_3_2_0 = "iOS_3_2_0",
	Android_3_2_1 = "Android_3_2_1",
	Linux_3_2_2__1 = "Linux_3_2_2__1",
	OS_X_3_2_2__1 = "OS_X_3_2_2__1",
	Windows_3_2_2__1 = "Windows_3_2_2__1",
	OS_X_3_2_2__2 = "OS_X_3_2_2__2",
	Windows_3_2_2__2 = "Windows_3_2_2__2",
	Android_3_2_2 = "Android_3_2_2",
	Windows_3_2_2__3 = "Windows_3_2_2__3",
	iOS_3_2_2 = "iOS_3_2_2",
	Windows_3_2_2__4 = "Windows_3_2_2__4",
	Linux_3_2_2__2 = "Linux_3_2_2__2",
	OS_X_3_2_2__3 = "OS_X_3_2_2__3",
	Windows_3_2_2__5 = "Windows_3_2_2__5",
	Android_3_2_3 = "Android_3_2_3",
	Android_3_2_4 = "Android_3_2_4",
	Linux_3_2_3 = "Linux_3_2_3",
	OS_X_3_2_3 = "OS_X_3_2_3",
	Windows_3_2_3 = "Windows_3_2_3",
	iOS_3_2_3 = "iOS_3_2_3",
	Windows_0_0_1__1 = "Windows_0_0_1__1",
	Windows_0_0_1__2 = "Windows_0_0_1__2",
	Windows_0_0_1__3 = "Windows_0_0_1__3",
	Linux_0_0_1__1 = "Linux_0_0_1__1",
	OS_X_0_0_1__1 = "OS_X_0_0_1__1",
	Linux_0_0_1__2 = "Linux_0_0_1__2",
	OS_X_0_0_1__2 = "OS_X_0_0_1__2",
	Windows_0_0_1__4 = "Windows_0_0_1__4",
	OS_X_0_0_1__3 = "OS_X_0_0_1__3",
	Windows_0_0_1__5 = "Windows_0_0_1__5",
	OS_X_0_0_1__4 = "OS_X_0_0_1__4",
	Windows_0_0_1__6 = "Windows_0_0_1__6",
	OS_X_0_0_1__5 = "OS_X_0_0_1__5",
	OS_X_0_0_1__6 = "OS_X_0_0_1__6",
	OS_X_0_0_1__7 = "OS_X_0_0_1__7",
	Linux_0_0_1__3 = "Linux_0_0_1__3",
	OS_X_0_0_1__8 = "OS_X_0_0_1__8",
	Windows_0_0_1__7 = "Windows_0_0_1__7",
	OS_X_0_0_1__9 = "OS_X_0_0_1__9",
	Windows_0_0_1__8 = "Windows_0_0_1__8",
	OS_X_0_0_1__10 = "OS_X_0_0_1__10",
	OS_X_0_0_1__11 = "OS_X_0_0_1__11",
	OS_X_0_0_1__12 = "OS_X_0_0_1__12",
	OS_X_0_0_1__13 = "OS_X_0_0_1__13",
	Windows_0_0_1__9 = "Windows_0_0_1__9",
	OS_X_0_0_1__14 = "OS_X_0_0_1__14",
	Windows_0_0_1__10 = "Windows_0_0_1__10",
	OS_X_0_0_1__15 = "OS_X_0_0_1__15",
	Windows_0_0_1__11 = "Windows_0_0_1__11",
	OS_X_0_0_1__16 = "OS_X_0_0_1__16",
	Windows_0_0_1__12 = "Windows_0_0_1__12",
	OS_X_0_0_1__17 = "OS_X_0_0_1__17",
	OS_X_0_0_1__18 = "OS_X_0_0_1__18",
	Windows_0_0_1__13 = "Windows_0_0_1__13",
	OS_X_0_0_1__19 = "OS_X_0_0_1__19",
	OS_X_0_0_1__20 = "OS_X_0_0_1__20",
	Windows_0_0_1__14 = "Windows_0_0_1__14",
	OS_X_0_0_1__21 = "OS_X_0_0_1__21",
	Windows_0_0_1__15 = "Windows_0_0_1__15",
	OS_X_0_0_1__22 = "OS_X_0_0_1__22",
	OS_X_0_0_1__23 = "OS_X_0_0_1__23",
	Windows_0_0_1__16 = "Windows_0_0_1__16",
	OS_X_0_0_1__24 = "OS_X_0_0_1__24",
	OS_X_0_0_1__25 = "OS_X_0_0_1__25",
	OS_X_0_0_1__26 = "OS_X_0_0_1__26",
	Windows_0_0_1__17 = "Windows_0_0_1__17",
	OS_X_0_0_1__27 = "OS_X_0_0_1__27",
	Windows_0_0_1__18 = "Windows_0_0_1__18",
	Windows_0_0_1__19 = "Windows_0_0_1__19",
	Android_3_2_5 = "Android_3_2_5",
	Windows_0_0_1__20 = "Windows_0_0_1__20",
	Windows_0_0_1__21 = "Windows_0_0_1__21",
	OS_X_0_0_1__28 = "OS_X_0_0_1__28",
	macOS_5_0_0_355a06f9 = "macOS_5_0_0_355a06f9",
	macOS_5_0_0_test_192 = "macOS_5_0_0_test_192",
	Windows_5_0_0_test_192 = "Windows_5_0_0_test_192",
	macOS_5_0_0_bf7671e7 = "macOS_5_0_0_bf7671e7",
	Windows_5_0_0_test_197 = "Windows_5_0_0_test_197",
	macOS_5_0_0_718a441b = "macOS_5_0_0_718a441b",
	Linux_3_2_5 = "Linux_3_2_5",
	OS_X_3_2_5 = "OS_X_3_2_5",
	Windows_3_2_5 = "Windows_3_2_5",
	macOS_5_0_0_59fc92f9 = "macOS_5_0_0_59fc92f9",
	OS_X_3_3_0__1 = "OS_X_3_3_0__1",
	Windows_3_3_0__1 = "Windows_3_3_0__1",
	macOS_5_0_0_ffa1a6e8 = "macOS_5_0_0_ffa1a6e8",
	macOS_5_0_0_01c1042e = "macOS_5_0_0_01c1042e",
	Windows_3_3_0__2 = "Windows_3_3_0__2",
	Windows_3_3_0__3 = "Windows_3_3_0__3",
	macOS_5_0_0_20262827 = "macOS_5_0_0_20262827",
	Windows_5_0_0_test_202 = "Windows_5_0_0_test_202",
	OS_X_3_3_0__2 = "OS_X_3_3_0__2",
	macOS_5_0_0_alpha203 = "macOS_5_0_0_alpha203",
	macOS_5_0_0_alpha204 = "macOS_5_0_0_alpha204",
	Windows_5_0_0_alpha204 = "Windows_5_0_0_alpha204",
	Linux_5_0_0_alpha206 = "Linux_5_0_0_alpha206",
	macOS_5_0_0_alpha206 = "macOS_5_0_0_alpha206",
	Windows_5_0_0_alpha206 = "Windows_5_0_0_alpha206",
	OS_X_3_3_0__3 = "OS_X_3_3_0__3",
	Windows_3_3_0__4 = "Windows_3_3_0__4",
	macOS_5_0_0_alpha207 = "macOS_5_0_0_alpha207",
	Windows_5_0_0_alpha207 = "Windows_5_0_0_alpha207",
	OS_X_3_3_0__4 = "OS_X_3_3_0__4",
	Windows_3_3_0__5 = "Windows_3_3_0__5",
	Windows_5_0_0_alpha208 = "Windows_5_0_0_alpha208",
	Windows_5_0_0_alpha209 = "Windows_5_0_0_alpha209",
	macOS_5_0_0_alpha212 = "macOS_5_0_0_alpha212",
	Windows_5_0_0_alpha212 = "Windows_5_0_0_alpha212",
	macOS_5_0_0_alpha214 = "macOS_5_0_0_alpha214",
	Windows_5_0_0_alpha214 = "Windows_5_0_0_alpha214",
	macOS_5_0_0_alpha216 = "macOS_5_0_0_alpha216",
	Windows_5_0_0_alpha216 = "Windows_5_0_0_alpha216",
	Windows_5_0_0_alpha217 = "Windows_5_0_0_alpha217",
	Windows_5_0_0_alpha218 = "Windows_5_0_0_alpha218",
	macOS_5_0_0_alpha219 = "macOS_5_0_0_alpha219",
	Windows_5_0_0_alpha219 = "Windows_5_0_0_alpha219",
	macOS_5_0_0_alpha220 = "macOS_5_0_0_alpha220",
	Windows_5_0_0_alpha220 = "Windows_5_0_0_alpha220",
	macOS_5_0_0_alpha222 = "macOS_5_0_0_alpha222",
	Windows_5_0_0_alpha222 = "Windows_5_0_0_alpha222",
	macOS_5_0_0_alpha223 = "macOS_5_0_0_alpha223",
	Windows_5_0_0_alpha223 = "Windows_5_0_0_alpha223",
	OS_X_3_3_0__5 = "OS_X_3_3_0__5",
	Windows_3_3_0__6 = "Windows_3_3_0__6",
	macOS_5_0_0_alpha224 = "macOS_5_0_0_alpha224",
	Windows_5_0_0_alpha225 = "Windows_5_0_0_alpha225",
	macOS_5_0_0_alpha226 = "macOS_5_0_0_alpha226",
	macOS_5_0_0_alpha228 = "macOS_5_0_0_alpha228",
	Windows_5_0_0_alpha228 = "Windows_5_0_0_alpha228",
	macOS_5_0_0_alpha229 = "macOS_5_0_0_alpha229",
	Windows_5_0_0_alpha229 = "Windows_5_0_0_alpha229",
	macOS_5_0_0_alpha230 = "macOS_5_0_0_alpha230",
	Windows_5_0_0_alpha230 = "Windows_5_0_0_alpha230",
	macOS_5_0_0_07b4003d = "macOS_5_0_0_07b4003d",
	Linux_5_0_0_alpha231 = "Linux_5_0_0_alpha231",
	macOS_5_0_0_alpha231 = "macOS_5_0_0_alpha231",
	Linux_5_0_0_alpha232 = "Linux_5_0_0_alpha232",
	macOS_5_0_0_alpha232 = "macOS_5_0_0_alpha232",
	Windows_5_0_0_alpha232 = "Windows_5_0_0_alpha232",
	OS_X_3_3_0__6 = "OS_X_3_3_0__6",
	Windows_3_3_0__7 = "Windows_3_3_0__7",
	Linux_3_3_0__1 = "Linux_3_3_0__1",
	OS_X_3_3_0__7 = "OS_X_3_3_0__7",
	OS_X_3_3_0__8 = "OS_X_3_3_0__8",
	OS_X_3_3_0__9 = "OS_X_3_3_0__9",
	iOS_3_3_0 = "iOS_3_3_0",
	OS_X_3_3_0__10 = "OS_X_3_3_0__10",
	Windows_3_3_0__8 = "Windows_3_3_0__8",
	OS_X_3_3_0__11 = "OS_X_3_3_0__11",
	Linux_3_3_0__2 = "Linux_3_3_0__2",
	OS_X_3_3_0__12 = "OS_X_3_3_0__12",
	Windows_3_3_0__9 = "Windows_3_3_0__9",
	macOS_5_0_0_alpha234 = "macOS_5_0_0_alpha234",
	Windows_5_0_0_alpha234 = "Windows_5_0_0_alpha234",
	Linux_3_3_0__3 = "Linux_3_3_0__3",
	OS_X_3_3_0__13 = "OS_X_3_3_0__13",
	Windows_3_3_0__10 = "Windows_3_3_0__10",
	OS_X_3_3_0__14 = "OS_X_3_3_0__14",
	Windows_3_3_0__11 = "Windows_3_3_0__11",
	Linux_3_3_0__4 = "Linux_3_3_0__4",
	OS_X_3_3_0__15 = "OS_X_3_3_0__15",
	Windows_3_3_0__12 = "Windows_3_3_0__12",
	macOS_5_0_0_alpha235 = "macOS_5_0_0_alpha235",
	Windows_5_0_0_alpha236 = "Windows_5_0_0_alpha236",
	macOS_5_0_0_alpha238 = "macOS_5_0_0_alpha238",
	Windows_5_0_0_alpha238 = "Windows_5_0_0_alpha238",
	OS_X_3_3_0__16 = "OS_X_3_3_0__16",
	Windows_5_0_0_alpha239 = "Windows_5_0_0_alpha239",
	OS_X_3_3_0__17 = "OS_X_3_3_0__17",
	Windows_3_3_0__13 = "Windows_3_3_0__13",
	Android_3_3_0 = "Android_3_3_0",
	macOS_5_0_0_alpha241 = "macOS_5_0_0_alpha241",
	Windows_5_0_0_alpha241 = "Windows_5_0_0_alpha241",
	Linux_3_3_0__5 = "Linux_3_3_0__5",
	OS_X_3_3_0__18 = "OS_X_3_3_0__18",
	Windows_3_3_0__14 = "Windows_3_3_0__14",
	Linux_3_3_0__6 = "Linux_3_3_0__6",
	OS_X_3_3_0__19 = "OS_X_3_3_0__19",
	Windows_3_3_0__15 = "Windows_3_3_0__15",
	macOS_5_0_0_alpha242 = "macOS_5_0_0_alpha242",
	Windows_5_0_0_alpha242 = "Windows_5_0_0_alpha242",
	OS_X_3_3_0__20 = "OS_X_3_3_0__20",
	Windows_3_3_0__16 = "Windows_3_3_0__16",
	OS_X_3_3_0__21 = "OS_X_3_3_0__21",
	Windows_3_3_0__17 = "Windows_3_3_0__17",
	macOS_5_0_0_alpha243 = "macOS_5_0_0_alpha243",
	Windows_5_0_0_alpha243 = "Windows_5_0_0_alpha243",
	Windows_3_3_0__18 = "Windows_3_3_0__18",
	OS_X_3_3_0__22 = "OS_X_3_3_0__22",
	OS_X_3_3_0__23 = "OS_X_3_3_0__23",
	Linux_3_3_0__7 = "Linux_3_3_0__7",
	OS_X_3_3_0__24 = "OS_X_3_3_0__24",
	Windows_3_3_0__19 = "Windows_3_3_0__19",
	macOS_5_0_0_alpha247 = "macOS_5_0_0_alpha247",
	Windows_5_0_0_alpha247 = "Windows_5_0_0_alpha247",
	macOS_5_0_0_alpha248 = "macOS_5_0_0_alpha248",
	Linux_3_3_1__1 = "Linux_3_3_1__1",
	OS_X_3_3_1__1 = "OS_X_3_3_1__1",
	Windows_3_3_1__1 = "Windows_3_3_1__1",
	macOS_5_0_0_alpha249 = "macOS_5_0_0_alpha249",
	Windows_5_0_0_alpha249 = "Windows_5_0_0_alpha249",
	OS_X_3_3_1__2 = "OS_X_3_3_1__2",
	macOS_5_0_0_alpha252 = "macOS_5_0_0_alpha252",
	macOS_5_0_0_alpha253 = "macOS_5_0_0_alpha253",
	OS_X_3_3_1__3 = "OS_X_3_3_1__3",
	Windows_3_3_1__2 = "Windows_3_3_1__2",
	Windows_5_0_0_alpha254 = "Windows_5_0_0_alpha254",
	Linux_3_3_1__2 = "Linux_3_3_1__2",
	OS_X_3_3_1__4 = "OS_X_3_3_1__4",
	Windows_3_3_1__3 = "Windows_3_3_1__3",
	Windows_5_0_0_alpha257 = "Windows_5_0_0_alpha257",
	Windows_5_0_0_alpha258 = "Windows_5_0_0_alpha258",
	Windows_5_0_0_alpha259 = "Windows_5_0_0_alpha259",
	Windows_5_0_0_alpha260 = "Windows_5_0_0_alpha260",
	Windows_5_0_0_alpha261 = "Windows_5_0_0_alpha261",
	Windows_5_0_0_alpha262 = "Windows_5_0_0_alpha262",
	Windows_5_0_0_alpha263 = "Windows_5_0_0_alpha263",
	Windows_5_0_0_alpha264 = "Windows_5_0_0_alpha264",
	Linux_5_0_0_alpha265 = "Linux_5_0_0_alpha265",
	Windows_5_0_0_alpha265 = "Windows_5_0_0_alpha265",
	Windows_5_0_0_alpha266 = "Windows_5_0_0_alpha266",
	OS_X_3_3_1__5 = "OS_X_3_3_1__5",
	OS_X_3_3_1__6 = "OS_X_3_3_1__6",
	Windows_5_0_0_alpha_267 = "Windows_5_0_0_alpha_267",
	macOS_5_0_0_alpha268 = "macOS_5_0_0_alpha268",
	Windows_5_0_0_alpha268 = "Windows_5_0_0_alpha268",
	Windows_5_0_0_alpha269 = "Windows_5_0_0_alpha269",
	Windows_5_0_0_alpha275 = "Windows_5_0_0_alpha275",
	Windows_5_0_0_max_updater_test_3 = "Windows_5_0_0_max_updater_test_3",
	Windows_5_0_0_alpha279 = "Windows_5_0_0_alpha279",
	macOS_5_0_0_alpha282 = "macOS_5_0_0_alpha282",
	macOS_5_0_0_alpha283 = "macOS_5_0_0_alpha283",
	Windows_5_0_0_alpha283 = "Windows_5_0_0_alpha283",
	OS_X_3_3_1__7 = "OS_X_3_3_1__7",
	OS_X_3_3_1__8 = "OS_X_3_3_1__8",
	Windows_3_3_1__4 = "Windows_3_3_1__4",
	Windows_5_0_0_alex_scroll_test_1 = "Windows_5_0_0_alex_scroll_test_1",
	OS_X_3_3_1__9 = "OS_X_3_3_1__9",
	macOS_5_0_0_alpha291 = "macOS_5_0_0_alpha291",
	Windows_5_0_0_alpha291 = "Windows_5_0_0_alpha291",
	macOS_5_0_0_alpha292 = "macOS_5_0_0_alpha292",
	Windows_5_0_0_alpha292 = "Windows_5_0_0_alpha292",
	Windows_5_0_0_alpha293 = "Windows_5_0_0_alpha293",
	Windows_5_0_0_alpha295 = "Windows_5_0_0_alpha295",
	Android_3_3_1__1 = "Android_3_3_1__1",
	Android_3_3_1__2 = "Android_3_3_1__2",
	OS_X_3_3_1__10 = "OS_X_3_3_1__10",
	macOS_5_0_0_alpha300 = "macOS_5_0_0_alpha300",
	Windows_5_0_0_alpha300 = "Windows_5_0_0_alpha300",
	Windows_5_0_0_alpha302 = "Windows_5_0_0_alpha302",
	Windows_5_0_0_alpha304 = "Windows_5_0_0_alpha304",
	macOS_5_0_0_alpha306 = "macOS_5_0_0_alpha306",
	Windows_5_0_0_alpha308 = "Windows_5_0_0_alpha308",
	OS_X_3_3_1__11 = "OS_X_3_3_1__11",
	Windows_5_0_0_alpha312 = "Windows_5_0_0_alpha312",
	Windows_5_0_0_alpha313 = "Windows_5_0_0_alpha313",
	Windows_5_0_0_alpha314 = "Windows_5_0_0_alpha314",
	Windows_3_5_0__1 = "Windows_3_5_0__1",
	Windows_5_0_0_alpha316 = "Windows_5_0_0_alpha316",
	Windows_5_0_0_alpha317 = "Windows_5_0_0_alpha317",
	OS_X_3_5_0__1 = "OS_X_3_5_0__1",
	macOS_5_0_0_alpha319 = "macOS_5_0_0_alpha319",
	Windows_5_0_0_alpha319 = "Windows_5_0_0_alpha319",
	Linux_3_3_1__3 = "Linux_3_3_1__3",
	OS_X_3_3_1__12 = "OS_X_3_3_1__12",
	Windows_3_3_1__5 = "Windows_3_3_1__5",
	Linux_3_3_2 = "Linux_3_3_2",
	OS_X_3_3_2 = "OS_X_3_3_2",
	Windows_3_3_2 = "Windows_3_3_2",
	Windows_3_5_0__2 = "Windows_3_5_0__2",
	Windows_5_0_0_alpha324 = "Windows_5_0_0_alpha324",
	OS_X_3_5_0__2 = "OS_X_3_5_0__2",
	Windows_3_5_0__3 = "Windows_3_5_0__3",
	Windows_5_0_0_alpha325 = "Windows_5_0_0_alpha325",
	Windows_5_0_0_alpha327 = "Windows_5_0_0_alpha327",
	Windows_5_0_0_alpha329 = "Windows_5_0_0_alpha329",
	Windows_5_0_0_cef_77_test_9 = "Windows_5_0_0_cef_77_test_9",
	Windows_5_0_0_alpha330 = "Windows_5_0_0_alpha330",
	Windows_5_0_0_alpha332 = "Windows_5_0_0_alpha332",
	Windows_5_0_0_alpha333 = "Windows_5_0_0_alpha333",
	Windows_5_0_0_alpha334 = "Windows_5_0_0_alpha334",
	Windows_5_0_0_alpha335 = "Windows_5_0_0_alpha335",
	Windows_5_0_0_alpha336 = "Windows_5_0_0_alpha336",
	Windows_3_5_0__4 = "Windows_3_5_0__4",
	Windows_5_0_0_alpha337 = "Windows_5_0_0_alpha337",
	Windows_5_0_0_alpha338 = "Windows_5_0_0_alpha338",
	OS_X_3_5_0__3 = "OS_X_3_5_0__3",
	Windows_3_5_0__5 = "Windows_3_5_0__5",
	Windows_5_0_0_alpha339 = "Windows_5_0_0_alpha339",
	Windows_5_0_0_alpha340 = "Windows_5_0_0_alpha340",
	macOS_5_0_0_alpha341 = "macOS_5_0_0_alpha341",
	Windows_5_0_0_alpha341 = "Windows_5_0_0_alpha341",
	Windows_5_0_0_alpha342 = "Windows_5_0_0_alpha342",
	Windows_5_0_0_alpha343 = "Windows_5_0_0_alpha343",
	Windows_5_0_0_alpha345 = "Windows_5_0_0_alpha345",
	Windows_3_5_0__6 = "Windows_3_5_0__6",
	Windows_5_0_0_alpha347 = "Windows_5_0_0_alpha347",
	Windows_5_0_0_alpha348 = "Windows_5_0_0_alpha348",
	macOS_5_0_0_alpha349 = "macOS_5_0_0_alpha349",
	Linux_3_5_0__1 = "Linux_3_5_0__1",
	OS_X_3_5_0__4 = "OS_X_3_5_0__4",
	Windows_3_5_0__7 = "Windows_3_5_0__7",
	Windows_5_0_0_alpha352 = "Windows_5_0_0_alpha352",
	macOS_5_0_0_alpha353 = "macOS_5_0_0_alpha353",
	Windows_5_0_0_alpha354 = "Windows_5_0_0_alpha354",
	Windows_5_0_0_alpha355 = "Windows_5_0_0_alpha355",
	Windows_5_0_0_alpha356 = "Windows_5_0_0_alpha356",
	macOS_5_0_0_alpha357 = "macOS_5_0_0_alpha357",
	Windows_5_0_0_alpha357 = "Windows_5_0_0_alpha357",
	Windows_5_0_0_alpha362 = "Windows_5_0_0_alpha362",
	Windows_5_0_0_alpha366 = "Windows_5_0_0_alpha366",
	Linux_5_0_0_alpha369 = "Linux_5_0_0_alpha369",
	macOS_5_0_0_alpha369 = "macOS_5_0_0_alpha369",
	Windows_5_0_0_alpha369 = "Windows_5_0_0_alpha369",
	Linux_5_0_0_alpha370 = "Linux_5_0_0_alpha370",
	macOS_5_0_0_alpha370 = "macOS_5_0_0_alpha370",
	Windows_5_0_0_alpha370 = "Windows_5_0_0_alpha370",
	macOS_5_0_0_alpha373 = "macOS_5_0_0_alpha373",
	Windows_5_0_0_alpha376 = "Windows_5_0_0_alpha376",
	macOS_5_0_0_alpha377 = "macOS_5_0_0_alpha377",
	Windows_5_0_0_alpha377 = "Windows_5_0_0_alpha377",
	macOS_5_0_0_alpha379 = "macOS_5_0_0_alpha379",
	Windows_5_0_0_alpha379 = "Windows_5_0_0_alpha379",
	macOS_5_0_0_alpha380 = "macOS_5_0_0_alpha380",
	Windows_5_0_0_alpha381 = "Windows_5_0_0_alpha381",
	Windows_5_0_0_alpha_383 = "Windows_5_0_0_alpha_383",
	Windows_5_0_0_alpha_384 = "Windows_5_0_0_alpha_384",
	Linux_5_0_0_alpha_385 = "Linux_5_0_0_alpha_385",
	macOS_5_0_0_alpha_385 = "macOS_5_0_0_alpha_385",
	Windows_5_0_0_alpha_385 = "Windows_5_0_0_alpha_385",
	macOS_5_0_0_alpha_389 = "macOS_5_0_0_alpha_389",
	Windows_5_0_0_alpha_389 = "Windows_5_0_0_alpha_389",
	Windows_5_0_0_alpha_390 = "Windows_5_0_0_alpha_390",
	iOS_3_5_0__1 = "iOS_3_5_0__1",
	OS_X_3_5_0__5 = "OS_X_3_5_0__5",
	Windows_3_5_0__8 = "Windows_3_5_0__8",
	Windows_5_0_0_alpha_392 = "Windows_5_0_0_alpha_392",
	Windows_5_0_0_alpha_393 = "Windows_5_0_0_alpha_393",
	Windows_5_0_0_alpha_395 = "Windows_5_0_0_alpha_395",
	Linux_5_0_0_beta_1 = "Linux_5_0_0_beta_1",
	macOS_5_0_0_beta_1 = "macOS_5_0_0_beta_1",
	Windows_5_0_0_beta_1 = "Windows_5_0_0_beta_1",
	Linux_3_5_0__2 = "Linux_3_5_0__2",
	OS_X_3_5_0__6 = "OS_X_3_5_0__6",
	Windows_3_5_0__9 = "Windows_3_5_0__9",
	Linux_5_0_0_beta_2 = "Linux_5_0_0_beta_2",
	macOS_5_0_0_beta_2 = "macOS_5_0_0_beta_2",
	Windows_5_0_0_beta_2 = "Windows_5_0_0_beta_2",
	Windows_5_0_0_beta_3__1 = "Windows_5_0_0_beta_3__1",
	Linux_5_0_0_beta_3 = "Linux_5_0_0_beta_3",
	macOS_5_0_0_beta_3 = "macOS_5_0_0_beta_3",
	Windows_5_0_0_beta_3__2 = "Windows_5_0_0_beta_3__2",
	macOS_5_0_0_beta_5 = "macOS_5_0_0_beta_5",
	macOS_5_0_0_beta_6 = "macOS_5_0_0_beta_6",
	Windows_5_0_0_mute_test_1 = "Windows_5_0_0_mute_test_1",
	Linux_5_0_0_beta_7 = "Linux_5_0_0_beta_7",
	macOS_5_0_0_beta_7 = "macOS_5_0_0_beta_7",
	Windows_5_0_0_beta_7 = "Windows_5_0_0_beta_7",
	Linux_5_0_0_beta_8 = "Linux_5_0_0_beta_8",
	macOS_5_0_0_beta_8 = "macOS_5_0_0_beta_8",
	Windows_5_0_0_beta_8 = "Windows_5_0_0_beta_8",
	Linux_5_0_0_beta_9 = "Linux_5_0_0_beta_9",
	macOS_5_0_0_beta_9 = "macOS_5_0_0_beta_9",
	Windows_5_0_0_beta_9 = "Windows_5_0_0_beta_9",
	macOS_5_0_0_beta_10 = "macOS_5_0_0_beta_10",
	Windows_5_0_0_beta_10 = "Windows_5_0_0_beta_10",
	OS_X_3_5_0__7 = "OS_X_3_5_0__7",
	Windows_5_0_0_inbox_update_test_3 = "Windows_5_0_0_inbox_update_test_3",
	OS_X_3_5_0__8 = "OS_X_3_5_0__8",
	Linux_5_0_0_017dbfce = "Linux_5_0_0_017dbfce",
	Linux_3_5_0__3 = "Linux_3_5_0__3",
	OS_X_3_5_0__9 = "OS_X_3_5_0__9",
	Windows_3_5_0__10 = "Windows_3_5_0__10",
	macOS_5_0_0_test_1 = "macOS_5_0_0_test_1",
	Windows_5_0_0_test_1 = "Windows_5_0_0_test_1",
	Linux_5_0_0_beta_12 = "Linux_5_0_0_beta_12",
	macOS_5_0_0_beta_12 = "macOS_5_0_0_beta_12",
	Windows_5_0_0_beta_12 = "Windows_5_0_0_beta_12",
	macOS_5_0_0_test_2 = "macOS_5_0_0_test_2",
	macOS_5_0_0_beta_13 = "macOS_5_0_0_beta_13",
	macOS_5_0_0_test_3 = "macOS_5_0_0_test_3",
	macOS_5_0_0_test_6 = "macOS_5_0_0_test_6",
	Windows_5_0_0_test_6 = "Windows_5_0_0_test_6",
	Windows_5_0_0_test_9 = "Windows_5_0_0_test_9",
	macOS_5_0_0_test_11 = "macOS_5_0_0_test_11",
	Windows_5_0_0_test_11 = "Windows_5_0_0_test_11",
	macOS_5_0_0_test_12 = "macOS_5_0_0_test_12",
	Windows_5_0_0_test_12 = "Windows_5_0_0_test_12",
	Windows_5_0_0_test_16 = "Windows_5_0_0_test_16",
	Linux_5_0_0_beta_15 = "Linux_5_0_0_beta_15",
	macOS_5_0_0_beta_15 = "macOS_5_0_0_beta_15",
	Windows_5_0_0_beta_15 = "Windows_5_0_0_beta_15",
	Windows_5_0_0_test_17 = "Windows_5_0_0_test_17",
	Linux_5_0_0_beta_16 = "Linux_5_0_0_beta_16",
	macOS_5_0_0_beta_16 = "macOS_5_0_0_beta_16",
	Windows_5_0_0_beta_16 = "Windows_5_0_0_beta_16",
	Linux_3_5_0__4 = "Linux_3_5_0__4",
	OS_X_3_5_0__10 = "OS_X_3_5_0__10",
	iOS_3_5_0__2 = "iOS_3_5_0__2",
	Windows_5_0_0_test_18 = "Windows_5_0_0_test_18",
	Windows_5_0_0_test_19 = "Windows_5_0_0_test_19",
	Windows_5_0_0_test_20 = "Windows_5_0_0_test_20",
	Linux_5_0_0_test_21 = "Linux_5_0_0_test_21",
	macOS_5_0_0_test_21 = "macOS_5_0_0_test_21",
	Windows_5_0_0_test_21 = "Windows_5_0_0_test_21",
	iOS_3_5_0__3 = "iOS_3_5_0__3",
	Windows_5_0_0_test_22 = "Windows_5_0_0_test_22",
	Windows_5_0_0_test_23 = "Windows_5_0_0_test_23",
	Android_3_5_0__1 = "Android_3_5_0__1",
	macOS_5_0_0_test_24 = "macOS_5_0_0_test_24",
	OS_X_3_5_0__11 = "OS_X_3_5_0__11",
	Windows_3_5_0__11 = "Windows_3_5_0__11",
	Windows_5_0_0_test_25 = "Windows_5_0_0_test_25",
	Linux_5_0_0_test_26 = "Linux_5_0_0_test_26",
	macOS_5_0_0_test_26 = "macOS_5_0_0_test_26",
	Windows_5_0_0_test_26 = "Windows_5_0_0_test_26",
	macOS_5_0_0_test_28 = "macOS_5_0_0_test_28",
	OS_X_3_5_0__12 = "OS_X_3_5_0__12",
	Windows_3_5_0__12 = "Windows_3_5_0__12",
	Windows_5_0_0_test_29 = "Windows_5_0_0_test_29",
	Windows_5_0_0_test_30 = "Windows_5_0_0_test_30",
	Linux_5_0_0_beta_17 = "Linux_5_0_0_beta_17",
	macOS_5_0_0_beta_17 = "macOS_5_0_0_beta_17",
	Windows_5_0_0_beta_17 = "Windows_5_0_0_beta_17",
	Windows_5_0_0_test_31 = "Windows_5_0_0_test_31",
	Windows_3_5_0__13 = "Windows_3_5_0__13",
	Windows_3_5_0__14 = "Windows_3_5_0__14",
	Windows_3_5_0__15 = "Windows_3_5_0__15",
	Windows_3_5_0__16 = "Windows_3_5_0__16",
	Windows_5_0_0_test_32 = "Windows_5_0_0_test_32",
	Linux_3_5_0__5 = "Linux_3_5_0__5",
	Windows_3_5_0__17 = "Windows_3_5_0__17",
	macOS_5_0_0_test_33 = "macOS_5_0_0_test_33",
	Windows_5_0_0_test_33 = "Windows_5_0_0_test_33",
	Linux_3_5_0__6 = "Linux_3_5_0__6",
	OS_X_3_5_0__13 = "OS_X_3_5_0__13",
	Windows_3_5_0__18 = "Windows_3_5_0__18",
	Linux_5_0_0_beta_18 = "Linux_5_0_0_beta_18",
	macOS_5_0_0_beta_18 = "macOS_5_0_0_beta_18",
	Windows_5_0_0_beta_18 = "Windows_5_0_0_beta_18",
	Windows_3_5_0__19 = "Windows_3_5_0__19",
	Windows_3_5_0__20 = "Windows_3_5_0__20",
	Windows_3_5_0__21 = "Windows_3_5_0__21",
	Windows_3_5_0__22 = "Windows_3_5_0__22",
	Linux_3_5_0__7 = "Linux_3_5_0__7",
	Windows_3_5_0__23 = "Windows_3_5_0__23",
	macOS_5_0_0_test_35 = "macOS_5_0_0_test_35",
	Windows_5_0_0_test_35 = "Windows_5_0_0_test_35",
	Linux_3_5_0__8 = "Linux_3_5_0__8",
	Windows_3_5_0__24 = "Windows_3_5_0__24",
	Windows_3_5_0__25 = "Windows_3_5_0__25",
	Windows_3_5_0__26 = "Windows_3_5_0__26",
	Linux_3_5_0__9 = "Linux_3_5_0__9",
	Windows_3_5_0__27 = "Windows_3_5_0__27",
	Windows_5_0_0_test_38 = "Windows_5_0_0_test_38",
	Linux_3_5_0__10 = "Linux_3_5_0__10",
	Windows_3_5_0__28 = "Windows_3_5_0__28",
	Linux_3_5_0__11 = "Linux_3_5_0__11",
	OS_X_3_5_0__14 = "OS_X_3_5_0__14",
	Windows_3_5_0__29 = "Windows_3_5_0__29",
	Windows_5_0_0_test_51 = "Windows_5_0_0_test_51",
	Windows_5_0_0_test_52 = "Windows_5_0_0_test_52",
	Windows_3_5_0__30 = "Windows_3_5_0__30",
	Windows_5_0_0_test_55 = "Windows_5_0_0_test_55",
	Linux_3_5_0__12 = "Linux_3_5_0__12",
	OS_X_3_5_0__15 = "OS_X_3_5_0__15",
	Windows_3_5_0__31 = "Windows_3_5_0__31",
	Windows_5_0_0_test_56 = "Windows_5_0_0_test_56",
	Windows_5_0_0_test_57 = "Windows_5_0_0_test_57",
	Linux_5_0_0_beta_19 = "Linux_5_0_0_beta_19",
	macOS_5_0_0_beta_19 = "macOS_5_0_0_beta_19",
	Windows_5_0_0_beta_19 = "Windows_5_0_0_beta_19",
	macOS_5_0_0_test_58 = "macOS_5_0_0_test_58",
	Windows_5_0_0_test_58 = "Windows_5_0_0_test_58",
	Windows_3_5_0__32 = "Windows_3_5_0__32",
	Linux_3_5_0__13 = "Linux_3_5_0__13",
	OS_X_3_5_0__16 = "OS_X_3_5_0__16",
	Windows_3_5_0__33 = "Windows_3_5_0__33",
	macOS_5_0_0_test_59 = "macOS_5_0_0_test_59",
	Windows_5_0_0_test_59 = "Windows_5_0_0_test_59",
	Linux_3_5_0__14 = "Linux_3_5_0__14",
	OS_X_3_5_0__17 = "OS_X_3_5_0__17",
	Windows_3_5_0__34 = "Windows_3_5_0__34",
	Windows_3_5_0__35 = "Windows_3_5_0__35",
	Windows_3_5_0__36 = "Windows_3_5_0__36",
	Windows_5_0_0_test_60 = "Windows_5_0_0_test_60",
	macOS_5_0_0_test_63 = "macOS_5_0_0_test_63",
	Windows_3_5_0__37 = "Windows_3_5_0__37",
	Windows_5_0_0_test_65 = "Windows_5_0_0_test_65",
	Linux_5_0_0_beta_20 = "Linux_5_0_0_beta_20",
	macOS_5_0_0_beta_20 = "macOS_5_0_0_beta_20",
	Windows_5_0_0_beta_20 = "Windows_5_0_0_beta_20",
	Windows_5_0_0_beta_21__1 = "Windows_5_0_0_beta_21__1",
	Windows_5_0_0_test_67 = "Windows_5_0_0_test_67",
	Windows_5_0_0_test_68 = "Windows_5_0_0_test_68",
	Linux_5_0_0_beta_21 = "Linux_5_0_0_beta_21",
	macOS_5_0_0_beta_21 = "macOS_5_0_0_beta_21",
	Windows_5_0_0_beta_21__2 = "Windows_5_0_0_beta_21__2",
	Windows_3_5_0__38 = "Windows_3_5_0__38",
	Windows_3_5_0__39 = "Windows_3_5_0__39",
	Windows_3_5_0__40 = "Windows_3_5_0__40",
	Windows_3_5_0__41 = "Windows_3_5_0__41",
	Windows_3_5_0__42 = "Windows_3_5_0__42",
	macOS_5_0_0_test_70 = "macOS_5_0_0_test_70",
	Windows_5_0_0_test_70 = "Windows_5_0_0_test_70",
	Windows_5_0_0_test_71 = "Windows_5_0_0_test_71",
	OS_X_3_5_0__18 = "OS_X_3_5_0__18",
	Windows_3_5_0__43 = "Windows_3_5_0__43",
	Windows_5_0_0_test_73 = "Windows_5_0_0_test_73",
	Windows_3_5_0__44 = "Windows_3_5_0__44",
	Linux_5_0_0_beta_22 = "Linux_5_0_0_beta_22",
	macOS_5_0_0_beta_22 = "macOS_5_0_0_beta_22",
	Windows_5_0_0_beta_22 = "Windows_5_0_0_beta_22",
	Android_3_5_0__2 = "Android_3_5_0__2",
	iOS_3_5_1 = "iOS_3_5_1",
	Windows_3_5_0__45 = "Windows_3_5_0__45",
	Android_3_5_0__3 = "Android_3_5_0__3",
	Linux_3_5_0__15 = "Linux_3_5_0__15",
	OS_X_3_5_0__19 = "OS_X_3_5_0__19",
	Windows_3_5_0__46 = "Windows_3_5_0__46",
	Linux_5_0_0_test_79 = "Linux_5_0_0_test_79",
	Linux_5_0_0_beta_23 = "Linux_5_0_0_beta_23",
	macOS_5_0_0_beta_23 = "macOS_5_0_0_beta_23",
	Windows_5_0_0_beta_23 = "Windows_5_0_0_beta_23",
	Linux_5_0_0_test_87 = "Linux_5_0_0_test_87",
	Windows_5_0_0_test_87 = "Windows_5_0_0_test_87",
	Windows_5_0_0_test_89 = "Windows_5_0_0_test_89",
	Linux_3_5_0__16 = "Linux_3_5_0__16",
	OS_X_3_5_0__20 = "OS_X_3_5_0__20",
	Windows_3_5_0__47 = "Windows_3_5_0__47",
	Windows_5_0_0_beta_24__1 = "Windows_5_0_0_beta_24__1",
	OS_X_3_5_0__21 = "OS_X_3_5_0__21",
	Windows_3_5_0__48 = "Windows_3_5_0__48",
	Linux_3_5_0__17 = "Linux_3_5_0__17",
	OS_X_3_5_0__22 = "OS_X_3_5_0__22",
	Windows_3_5_0__49 = "Windows_3_5_0__49",
	Linux_5_0_0_beta_24 = "Linux_5_0_0_beta_24",
	macOS_5_0_0_beta_24 = "macOS_5_0_0_beta_24",
	Windows_5_0_0_beta_24__2 = "Windows_5_0_0_beta_24__2",
	Windows_3_5_1__1 = "Windows_3_5_1__1",
	Windows_3_5_1__2 = "Windows_3_5_1__2",
	Linux_3_5_1 = "Linux_3_5_1",
	OS_X_3_5_1 = "OS_X_3_5_1",
	Windows_3_5_1__3 = "Windows_3_5_1__3",
	Windows_5_0_0_test_95 = "Windows_5_0_0_test_95",
	Windows_5_0_0_test_96 = "Windows_5_0_0_test_96",
	Linux_3_5_2__1 = "Linux_3_5_2__1",
	Windows_3_5_2__1 = "Windows_3_5_2__1",
	Windows_5_0_0_test_100 = "Windows_5_0_0_test_100",
	Linux_3_5_2__2 = "Linux_3_5_2__2",
	OS_X_3_5_2 = "OS_X_3_5_2",
	Windows_3_5_2__2 = "Windows_3_5_2__2",
	Windows_5_0_0_manage_permissions_8 = "Windows_5_0_0_manage_permissions_8",
	Windows_5_0_0_network_monitor_1 = "Windows_5_0_0_network_monitor_1",
	Linux_3_5_3__1 = "Linux_3_5_3__1",
	OS_X_3_5_3__1 = "OS_X_3_5_3__1",
	Windows_3_5_3__1 = "Windows_3_5_3__1",
	macOS_5_0_0_test_104 = "macOS_5_0_0_test_104",
	Windows_5_0_0_test_104 = "Windows_5_0_0_test_104",
	Windows_3_5_3__2 = "Windows_3_5_3__2",
	Windows_3_5_3__3 = "Windows_3_5_3__3",
	Linux_3_5_3__2 = "Linux_3_5_3__2",
	OS_X_3_5_3__2 = "OS_X_3_5_3__2",
	Windows_3_5_3__4 = "Windows_3_5_3__4",
	macOS_5_0_0_test_106 = "macOS_5_0_0_test_106",
	Linux_3_5_3__3 = "Linux_3_5_3__3",
	Windows_3_5_3__5 = "Windows_3_5_3__5",
	Windows_5_0_0_test_111 = "Windows_5_0_0_test_111",
	Linux_3_5_3__4 = "Linux_3_5_3__4",
	Windows_3_5_3__6 = "Windows_3_5_3__6",
	Linux_5_0_0_beta_25 = "Linux_5_0_0_beta_25",
	macOS_5_0_0_beta_25 = "macOS_5_0_0_beta_25",
	Windows_5_0_0_beta_25 = "Windows_5_0_0_beta_25",
	Windows_5_0_0_test_113 = "Windows_5_0_0_test_113",
	Windows_3_5_3__7 = "Windows_3_5_3__7",
	Windows_3_5_3__8 = "Windows_3_5_3__8",
	Linux_5_0_0_test_114 = "Linux_5_0_0_test_114",
	macOS_5_0_0_test_114 = "macOS_5_0_0_test_114",
	Windows_5_0_0_test_114 = "Windows_5_0_0_test_114",
	Linux_3_5_3__5 = "Linux_3_5_3__5",
	OS_X_3_5_3__3 = "OS_X_3_5_3__3",
	Windows_3_5_3__9 = "Windows_3_5_3__9",
	Windows_5_0_0_test_116 = "Windows_5_0_0_test_116",
	Windows_5_0_0_test_120 = "Windows_5_0_0_test_120",
	Linux_3_5_5__1 = "Linux_3_5_5__1",
	OS_X_3_5_5__1 = "OS_X_3_5_5__1",
	Windows_3_5_5__1 = "Windows_3_5_5__1",
	Linux_3_5_5__2 = "Linux_3_5_5__2",
	OS_X_3_5_5__2 = "OS_X_3_5_5__2",
	Windows_3_5_5__2 = "Windows_3_5_5__2",
	macOS_5_0_0_ui_polishing_2 = "macOS_5_0_0_ui_polishing_2",
	Windows_5_0_0_tschat_test_7 = "Windows_5_0_0_tschat_test_7",
	Windows_5_0_0_tschat_test_9 = "Windows_5_0_0_tschat_test_9",
	macOS_5_0_0_ui_polishing_4 = "macOS_5_0_0_ui_polishing_4",
	Windows_5_0_0_ui_polishing_4 = "Windows_5_0_0_ui_polishing_4",
	Windows_5_0_0_flag_test_3 = "Windows_5_0_0_flag_test_3",
	Windows_5_0_0_move_testing_1 = "Windows_5_0_0_move_testing_1",
	Windows_5_0_0_move_testing_2 = "Windows_5_0_0_move_testing_2",
	Windows_5_0_0_move_testing_4 = "Windows_5_0_0_move_testing_4",
	Windows_5_0_0_flag_test_4 = "Windows_5_0_0_flag_test_4",
	Windows_5_0_0_move_testing_7 = "Windows_5_0_0_move_testing_7",
	Windows_5_0_0_tschat_test_11 = "Windows_5_0_0_tschat_test_11",
	Windows_5_0_0_ui_polishing_5 = "Windows_5_0_0_ui_polishing_5",
	Windows_5_0_0_ui_polishing_6 = "Windows_5_0_0_ui_polishing_6",
	Windows_5_0_0_ui_polishing_7 = "Windows_5_0_0_ui_polishing_7",
	Windows_5_0_0_4f3ac28d8 = "Windows_5_0_0_4f3ac28d8",
	Windows_5_0_0_tschat_test_15 = "Windows_5_0_0_tschat_test_15",
	Windows_5_0_0_cobra_test_2 = "Windows_5_0_0_cobra_test_2",
	Windows_5_0_0_alpha400 = "Windows_5_0_0_alpha400",
	Windows_5_0_0_alpha401 = "Windows_5_0_0_alpha401",
	Windows_5_0_0_alpha402 = "Windows_5_0_0_alpha402",
	Windows_5_0_0_alpha403 = "Windows_5_0_0_alpha403",
	Windows_5_0_0_alpha404 = "Windows_5_0_0_alpha404",
	Android_3_5_0__4 = "Android_3_5_0__4",
	Windows_5_0_0_cobra_test_3 = "Windows_5_0_0_cobra_test_3",
	Windows_5_0_0_54c87064a = "Windows_5_0_0_54c87064a",
	Windows_5_0_0_alpha406 = "Windows_5_0_0_alpha406",
	Windows_5_0_0_cobra_test_4 = "Windows_5_0_0_cobra_test_4",
	Windows_5_0_0_alpha407 = "Windows_5_0_0_alpha407",
	Windows_5_0_0_alpha408 = "Windows_5_0_0_alpha408",
	Windows_5_0_0_updater_test_8 = "Windows_5_0_0_updater_test_8",
	Windows_5_0_0_alpha409 = "Windows_5_0_0_alpha409",
	Windows_5_0_0_alpha410 = "Windows_5_0_0_alpha410",
	Android_3_5_0__5 = "Android_3_5_0__5",
	macOS_5_0_0_alpha411 = "macOS_5_0_0_alpha411",
	Windows_5_0_0_alpha411 = "Windows_5_0_0_alpha411",
	Windows_5_0_0_alpha412 = "Windows_5_0_0_alpha412",
	macOS_5_0_0_beta26_rc1 = "macOS_5_0_0_beta26_rc1",
	Windows_5_0_0_beta26_rc1 = "Windows_5_0_0_beta26_rc1",
	Windows_5_0_0_beta26_rc2 = "Windows_5_0_0_beta26_rc2",
	Windows_5_0_0_beta26_rc3 = "Windows_5_0_0_beta26_rc3",
	macOS_5_0_0_beta26_rc4 = "macOS_5_0_0_beta26_rc4",
	Windows_5_0_0_beta26_rc4 = "Windows_5_0_0_beta26_rc4",
	macOS_5_0_0_beta26_rc5 = "macOS_5_0_0_beta26_rc5",
	Windows_5_0_0_beta26_rc5 = "Windows_5_0_0_beta26_rc5",
	macOS_5_0_0_beta26_rc6 = "macOS_5_0_0_beta26_rc6",
	Windows_5_0_0_beta26_rc6 = "Windows_5_0_0_beta26_rc6",
	Windows_5_0_0_beta26_rc7 = "Windows_5_0_0_beta26_rc7",
	Windows_5_0_0_beta26_rc8 = "Windows_5_0_0_beta26_rc8",
	Windows_5_0_0_beta26__1 = "Windows_5_0_0_beta26__1",
	Linux_5_0_0_beta26 = "Linux_5_0_0_beta26",
	macOS_5_0_0_beta26 = "macOS_5_0_0_beta26",
	Windows_5_0_0_beta26__2 = "Windows_5_0_0_beta26__2",
	Windows_5_0_0_cobra_test_5 = "Windows_5_0_0_cobra_test_5",
	Windows_5_0_0_cobra_test_6 = "Windows_5_0_0_cobra_test_6",
	macOS_5_0_0_beta27_rc1 = "macOS_5_0_0_beta27_rc1",
	Windows_5_0_0_beta27_rc1 = "Windows_5_0_0_beta27_rc1",
	Windows_5_0_0_beta27_rc2 = "Windows_5_0_0_beta27_rc2",
	Windows_5_0_0_beta27_rc3 = "Windows_5_0_0_beta27_rc3",
	Linux_5_0_0_beta27 = "Linux_5_0_0_beta27",
	macOS_5_0_0_beta27 = "macOS_5_0_0_beta27",
	Windows_5_0_0_beta27 = "Windows_5_0_0_beta27",
	Windows_5_0_0_cobra_test_7 = "Windows_5_0_0_cobra_test_7",
	Windows_5_0_0_cobra_test_10 = "Windows_5_0_0_cobra_test_10",
	Windows_5_0_0_cobra_test_11 = "Windows_5_0_0_cobra_test_11",
	macOS_5_0_0_beta28_rc1 = "macOS_5_0_0_beta28_rc1",
	Windows_5_0_0_beta28_rc1 = "Windows_5_0_0_beta28_rc1",
	Android_3_5_0__6 = "Android_3_5_0__6",
	macOS_5_0_0_beta28_rc2 = "macOS_5_0_0_beta28_rc2",
	Windows_5_0_0_beta28_rc2 = "Windows_5_0_0_beta28_rc2",
	Linux_5_0_0_beta28 = "Linux_5_0_0_beta28",
	macOS_5_0_0_beta28 = "macOS_5_0_0_beta28",
	Windows_5_0_0_beta28 = "Windows_5_0_0_beta28",
	Windows_5_0_0_beta29_rc1 = "Windows_5_0_0_beta29_rc1",
	macOS_5_0_0_beta29_rc2 = "macOS_5_0_0_beta29_rc2",
	Windows_5_0_0_beta29_rc3 = "Windows_5_0_0_beta29_rc3",
	macOS_5_0_0_beta29 = "macOS_5_0_0_beta29",
	Windows_5_0_0_beta29 = "Windows_5_0_0_beta29",
	Linux_5_0_0_beta29_1 = "Linux_5_0_0_beta29_1",
	macOS_5_0_0_beta29_1 = "macOS_5_0_0_beta29_1",
	Windows_5_0_0_beta29_1 = "Windows_5_0_0_beta29_1",
	macOS_5_0_0_beta30_rc1 = "macOS_5_0_0_beta30_rc1",
	Windows_5_0_0_beta30_rc1 = "Windows_5_0_0_beta30_rc1",
	Windows_5_0_0_beta30_rc2 = "Windows_5_0_0_beta30_rc2",
	Windows_5_0_0_beta30_rc3 = "Windows_5_0_0_beta30_rc3",
	Linux_5_0_0_beta30 = "Linux_5_0_0_beta30",
	macOS_5_0_0_beta30 = "macOS_5_0_0_beta30",
	Windows_5_0_0_beta30 = "Windows_5_0_0_beta30",
	Windows_5_0_0_internal_1 = "Windows_5_0_0_internal_1",
	Windows_3_5_5__3 = "Windows_3_5_5__3",
	Windows_5_0_0_beta31_rc1 = "Windows_5_0_0_beta31_rc1",
	Windows_5_0_0_beta31_rc2 = "Windows_5_0_0_beta31_rc2",
	Linux_5_0_0_beta31 = "Linux_5_0_0_beta31",
	macOS_5_0_0_beta31 = "macOS_5_0_0_beta31",
	Windows_5_0_0_beta31 = "Windows_5_0_0_beta31",
	Linux_3_5_5__3 = "Linux_3_5_5__3",
	OS_X_3_5_5__3 = "OS_X_3_5_5__3",
	Windows_3_5_5__4 = "Windows_3_5_5__4",
	Windows_5_0_0_beta32_rc1 = "Windows_5_0_0_beta32_rc1",
	macOS_5_0_0_beta32_rc2 = "macOS_5_0_0_beta32_rc2",
	Windows_5_0_0_beta32_rc2 = "Windows_5_0_0_beta32_rc2",
	macOS_5_0_0_beta32_rc3 = "macOS_5_0_0_beta32_rc3",
	Windows_5_0_0_beta32_rc4 = "Windows_5_0_0_beta32_rc4",
	Windows_5_0_0_beta32_rc5 = "Windows_5_0_0_beta32_rc5",
	Linux_5_0_0_beta32 = "Linux_5_0_0_beta32",
	macOS_5_0_0_beta32 = "macOS_5_0_0_beta32",
	Windows_5_0_0_beta32 = "Windows_5_0_0_beta32",
	Windows_5_0_0_bab0e5e53 = "Windows_5_0_0_bab0e5e53",
	iOS_3_5_6 = "iOS_3_5_6",
	Windows_5_0_0_cobra_testing_11 = "Windows_5_0_0_cobra_testing_11",
	macOS_5_0_0_beta33_rc1 = "macOS_5_0_0_beta33_rc1",
	Windows_5_0_0_beta33_rc1 = "Windows_5_0_0_beta33_rc1",
	macOS_5_0_0_test_icons = "macOS_5_0_0_test_icons",
	Android_3_5_0__7 = "Android_3_5_0__7",
	macOS_5_0_0_beta33_rc2 = "macOS_5_0_0_beta33_rc2",
	Windows_5_0_0_beta33_rc2 = "Windows_5_0_0_beta33_rc2",
	Linux_5_0_0_beta33 = "Linux_5_0_0_beta33",
	Windows_5_0_0_beta33 = "Windows_5_0_0_beta33",
	Windows_5_0_0_alpha414 = "Windows_5_0_0_alpha414",
	Windows_5_0_0_new_certificate_test_2 = "Windows_5_0_0_new_certificate_test_2",
	Windows_5_0_0_prev416 = "Windows_5_0_0_prev416",
	Windows_3_5_6__1 = "Windows_3_5_6__1",
	Windows_5_0_0_beta34_rc1__1 = "Windows_5_0_0_beta34_rc1__1",
	Windows_5_0_0_beta34_rc1__2 = "Windows_5_0_0_beta34_rc1__2",
	Windows_5_0_0_beta34_rc2__1 = "Windows_5_0_0_beta34_rc2__1",
	Windows_5_0_0_beta34_rc2__2 = "Windows_5_0_0_beta34_rc2__2",
	Linux_5_0_0_beta34 = "Linux_5_0_0_beta34",
	macOS_5_0_0_beta34 = "macOS_5_0_0_beta34",
	Windows_5_0_0_beta34 = "Windows_5_0_0_beta34",
	Linux_3_5_6 = "Linux_3_5_6",
	OS_X_3_5_6 = "OS_X_3_5_6",
	Windows_3_5_6__2 = "Windows_3_5_6__2",
	Windows_5_0_0_qa_1 = "Windows_5_0_0_qa_1",
	Windows_3_5_7__1 = "Windows_3_5_7__1",
	Windows_3_5_7__2 = "Windows_3_5_7__2",
	Linux_5_0_0_beta35_rc1 = "Linux_5_0_0_beta35_rc1",
	Windows_5_0_0_beta35_rc1 = "Windows_5_0_0_beta35_rc1",
	Windows_5_0_0_beta35_rc2 = "Windows_5_0_0_beta35_rc2",
	Linux_5_0_0_beta35 = "Linux_5_0_0_beta35",
	macOS_5_0_0_beta35 = "macOS_5_0_0_beta35",
	Windows_5_0_0_beta35 = "Windows_5_0_0_beta35",
	Windows_5_0_0_cobra_testing_12 = "Windows_5_0_0_cobra_testing_12",
	macOS_5_0_0_beta36_rc1 = "macOS_5_0_0_beta36_rc1",
	Windows_5_0_0_beta36_rc1 = "Windows_5_0_0_beta36_rc1",
	Linux_5_0_0_beta36_rc3 = "Linux_5_0_0_beta36_rc3",
	Windows_5_0_0_beta36_rc3 = "Windows_5_0_0_beta36_rc3",
	Linux_3_5_7__1 = "Linux_3_5_7__1",
	Windows_3_5_7__3 = "Windows_3_5_7__3",
	Windows_5_0_0_beta36_rc4 = "Windows_5_0_0_beta36_rc4",
	Linux_5_0_0_beta36 = "Linux_5_0_0_beta36",
	macOS_5_0_0_beta36 = "macOS_5_0_0_beta36",
	Windows_5_0_0_beta36 = "Windows_5_0_0_beta36",
	Windows_5_0_0_qa_4 = "Windows_5_0_0_qa_4",
	Windows_5_0_0_cobra_testing_14 = "Windows_5_0_0_cobra_testing_14",
	Windows_5_0_0_beta36_1_rc1 = "Windows_5_0_0_beta36_1_rc1",
	Linux_5_0_0_beta36_1 = "Linux_5_0_0_beta36_1",
	macOS_5_0_0_beta36_1 = "macOS_5_0_0_beta36_1",
	Windows_5_0_0_beta36_1 = "Windows_5_0_0_beta36_1",
	Windows_5_0_0_qa_8 = "Windows_5_0_0_qa_8",
	Windows_5_0_0_cobra_testing_15 = "Windows_5_0_0_cobra_testing_15",
	Linux_3_5_7__2 = "Linux_3_5_7__2",
	OS_X_3_5_7__1 = "OS_X_3_5_7__1",
	Windows_3_5_7__4 = "Windows_3_5_7__4",
	Windows_5_0_0_qa_12 = "Windows_5_0_0_qa_12",
	Windows_5_0_0_qa_13 = "Windows_5_0_0_qa_13",
	Windows_5_0_0_cobra_testing_16 = "Windows_5_0_0_cobra_testing_16",
	Windows_5_0_0_qa_16 = "Windows_5_0_0_qa_16",
	Linux_3_5_7__3 = "Linux_3_5_7__3",
	Windows_3_5_7__5 = "Windows_3_5_7__5",
	Windows_5_0_0_beta37_rc1 = "Windows_5_0_0_beta37_rc1",
	Windows_5_0_0_beta37_rc2 = "Windows_5_0_0_beta37_rc2",
	Linux_5_0_0_beta37 = "Linux_5_0_0_beta37",
	macOS_5_0_0_beta37 = "macOS_5_0_0_beta37",
	Windows_5_0_0_beta37 = "Windows_5_0_0_beta37",
	Windows_5_0_0_beta38_rc1 = "Windows_5_0_0_beta38_rc1",
	Windows_5_0_0_qa_18 = "Windows_5_0_0_qa_18",
	Windows_5_0_0_cobra_testing_17 = "Windows_5_0_0_cobra_testing_17",
	Windows_5_0_0_cobra_testing_18 = "Windows_5_0_0_cobra_testing_18",
	Windows_5_0_0_cobra_testing_19 = "Windows_5_0_0_cobra_testing_19",
	Windows_5_0_0_qa_19 = "Windows_5_0_0_qa_19",
	Windows_5_0_0_qa_20 = "Windows_5_0_0_qa_20",
	Windows_5_0_0_beta38_rc2 = "Windows_5_0_0_beta38_rc2",
	Windows_5_0_0_cobra_testing_20 = "Windows_5_0_0_cobra_testing_20",
	Windows_5_0_0_beta38_rc3 = "Windows_5_0_0_beta38_rc3",
	Linux_5_0_0_beta38 = "Linux_5_0_0_beta38",
	macOS_5_0_0_beta38 = "macOS_5_0_0_beta38",
	Windows_5_0_0_beta38 = "Windows_5_0_0_beta38",
	Windows_5_0_0_qa_21 = "Windows_5_0_0_qa_21",
	Windows_5_0_0_qa_23 = "Windows_5_0_0_qa_23",
	Windows_5_0_0_qa_24 = "Windows_5_0_0_qa_24",
	Windows_5_0_0_qa_25 = "Windows_5_0_0_qa_25",
	Windows_5_0_0_beta39_rc1 = "Windows_5_0_0_beta39_rc1",
	Linux_5_0_0_beta39 = "Linux_5_0_0_beta39",
	macOS_5_0_0_beta39 = "macOS_5_0_0_beta39",
	Windows_5_0_0_beta39 = "Windows_5_0_0_beta39",
	Windows_5_0_0_qa_26 = "Windows_5_0_0_qa_26",
	macOS_5_0_0_beta40_rc1 = "macOS_5_0_0_beta40_rc1",
	Windows_5_0_0_beta40_rc1 = "Windows_5_0_0_beta40_rc1",
	Windows_5_0_0_cobra_testing_21 = "Windows_5_0_0_cobra_testing_21",
	Windows_5_0_0_special_build = "Windows_5_0_0_special_build",
	Windows_5_0_0_pin_msg_redesign_8 = "Windows_5_0_0_pin_msg_redesign_8",
	Windows_5_0_0_beta40_rc2 = "Windows_5_0_0_beta40_rc2",
	Windows_5_0_0_beta40_rc3 = "Windows_5_0_0_beta40_rc3",
	Windows_5_0_0_ava_4 = "Windows_5_0_0_ava_4",
	Windows_5_0_0_beta40_rc5 = "Windows_5_0_0_beta40_rc5",
	macOS_5_0_0_beta40 = "macOS_5_0_0_beta40",
	Windows_5_0_0_beta40 = "Windows_5_0_0_beta40",
	Windows_5_0_0_qa_29 = "Windows_5_0_0_qa_29",
	Windows_5_0_0_beta41_rc1 = "Windows_5_0_0_beta41_rc1",
	Windows_5_0_0_beta41_rc2 = "Windows_5_0_0_beta41_rc2",
	Windows_3_5_7__6 = "Windows_3_5_7__6",
	Windows_5_0_0_beta41_rc3 = "Windows_5_0_0_beta41_rc3",
	Windows_5_0_0_beta41_rc4 = "Windows_5_0_0_beta41_rc4",
	Windows_5_0_0_qa_33 = "Windows_5_0_0_qa_33",
	Windows_5_0_0_beta41 = "Windows_5_0_0_beta41",
	Windows_5_0_0_pin_msg_redesign_9 = "Windows_5_0_0_pin_msg_redesign_9",
	Windows_5_0_0_beta41_1_rc2 = "Windows_5_0_0_beta41_1_rc2",
	OS_X_3_5_7__2 = "OS_X_3_5_7__2",
	Windows_3_5_7__7 = "Windows_3_5_7__7",
	Linux_5_0_0_beta41_1 = "Linux_5_0_0_beta41_1",
	macOS_5_0_0_beta41_1 = "macOS_5_0_0_beta41_1",
	Windows_5_0_0_beta41_1 = "Windows_5_0_0_beta41_1",
	Windows_5_0_0_qa_37 = "Windows_5_0_0_qa_37",
	Windows_5_0_0_qa_38 = "Windows_5_0_0_qa_38",
	Windows_5_0_0_beta42_rc1 = "Windows_5_0_0_beta42_rc1",
	Windows_5_0_0_qa_50 = "Windows_5_0_0_qa_50",
	Windows_5_0_0_beta42_rc2 = "Windows_5_0_0_beta42_rc2",
	Linux_5_0_0_beta42 = "Linux_5_0_0_beta42",
	macOS_5_0_0_beta42 = "macOS_5_0_0_beta42",
	Windows_5_0_0_beta42 = "Windows_5_0_0_beta42",
	Windows_5_0_0_whip_6 = "Windows_5_0_0_whip_6",
	Windows_5_0_0_beta43_rc1 = "Windows_5_0_0_beta43_rc1",
	Windows_5_0_0_beta43_rc2 = "Windows_5_0_0_beta43_rc2",
	Windows_5_0_0_beta43__1 = "Windows_5_0_0_beta43__1",
	Linux_5_0_0_beta43 = "Linux_5_0_0_beta43",
	macOS_5_0_0_beta43 = "macOS_5_0_0_beta43",
	Windows_5_0_0_beta43__2 = "Windows_5_0_0_beta43__2",
	Windows_5_0_0_beta44_rc1 = "Windows_5_0_0_beta44_rc1",
	Linux_5_0_0_beta44_rc2 = "Linux_5_0_0_beta44_rc2",
	Windows_5_0_0_beta44_rc2 = "Windows_5_0_0_beta44_rc2",
	Windows_5_0_0_beta44_rc3 = "Windows_5_0_0_beta44_rc3",
	Windows_5_0_0_whip_7 = "Windows_5_0_0_whip_7",
	Windows_5_0_0_beta44_rc7 = "Windows_5_0_0_beta44_rc7",
	Linux_5_0_0_beta44 = "Linux_5_0_0_beta44",
	macOS_5_0_0_beta44 = "macOS_5_0_0_beta44",
	Windows_5_0_0_beta44 = "Windows_5_0_0_beta44",
	Windows_5_0_0_whip_9 = "Windows_5_0_0_whip_9",
	Windows_5_0_0_whip_10 = "Windows_5_0_0_whip_10",
	Windows_5_0_0_beta45_rc1 = "Windows_5_0_0_beta45_rc1",
	OS_X_3_5_7__3 = "OS_X_3_5_7__3",
	Windows_5_0_0_beta45_rc2 = "Windows_5_0_0_beta45_rc2",
	Windows_5_0_0_beta45_rc3 = "Windows_5_0_0_beta45_rc3",
	Linux_5_0_0_beta45 = "Linux_5_0_0_beta45",
	Windows_5_0_0_beta45 = "Windows_5_0_0_beta45",
	macOS_5_0_0_beta45_1 = "macOS_5_0_0_beta45_1",
	Windows_5_0_0_beta45_1 = "Windows_5_0_0_beta45_1",
	Linux_5_0_0_beta45_2 = "Linux_5_0_0_beta45_2",
	macOS_5_0_0_beta45_2 = "macOS_5_0_0_beta45_2",
	Windows_5_0_0_beta45_2 = "Windows_5_0_0_beta45_2",
	Windows_5_0_0_beta46_rc1 = "Windows_5_0_0_beta46_rc1",
	Windows_5_0_0_beta46_rc2 = "Windows_5_0_0_beta46_rc2",
	Windows_5_0_0_beta46_rc3 = "Windows_5_0_0_beta46_rc3",
	Windows_5_0_0_cobra_testing_30 = "Windows_5_0_0_cobra_testing_30",
	Linux_5_0_0_beta46 = "Linux_5_0_0_beta46",
	macOS_5_0_0_beta46 = "macOS_5_0_0_beta46",
	Windows_5_0_0_beta46 = "Windows_5_0_0_beta46",
	Windows_5_0_0_beta47_rc1 = "Windows_5_0_0_beta47_rc1",
	Windows_5_0_0_beta47_rc2 = "Windows_5_0_0_beta47_rc2",
	Windows_5_0_0_co_1 = "Windows_5_0_0_co_1",
	Windows_5_0_0_beta47_rc3 = "Windows_5_0_0_beta47_rc3",
	Windows_5_0_0_beta47_rc4 = "Windows_5_0_0_beta47_rc4",
	Linux_5_0_0_beta47 = "Linux_5_0_0_beta47",
	macOS_5_0_0_beta47 = "macOS_5_0_0_beta47",
	Windows_5_0_0_beta47 = "Windows_5_0_0_beta47",
	Windows_5_0_0_qa_48_1 = "Windows_5_0_0_qa_48_1",
	Windows_5_0_0_beta48_rc1 = "Windows_5_0_0_beta48_rc1",
	Linux_5_0_0_beta48 = "Linux_5_0_0_beta48",
	macOS_5_0_0_beta48 = "macOS_5_0_0_beta48",
	Windows_5_0_0_beta48 = "Windows_5_0_0_beta48",
	Windows_5_0_0_beta49_rc1 = "Windows_5_0_0_beta49_rc1",
	Windows_5_0_0_ticket2000_2 = "Windows_5_0_0_ticket2000_2",
	Windows_5_0_0_beta49_rc2 = "Windows_5_0_0_beta49_rc2",
	Windows_5_0_0_ticket2000_3 = "Windows_5_0_0_ticket2000_3",
	Windows_5_0_0_t2000_4 = "Windows_5_0_0_t2000_4",
	Windows_5_0_0_beta49_rc3 = "Windows_5_0_0_beta49_rc3",
	Windows_5_0_0_qa_49_7 = "Windows_5_0_0_qa_49_7",
	Linux_5_0_0_beta49 = "Linux_5_0_0_beta49",
	macOS_5_0_0_beta49 = "macOS_5_0_0_beta49",
	Windows_5_0_0_beta49 = "Windows_5_0_0_beta49",
	Windows_5_0_0_shorturls_2 = "Windows_5_0_0_shorturls_2",
	Windows_5_0_0_beta50_rc2 = "Windows_5_0_0_beta50_rc2",
	macOS_5_0_0_beta50_rc3 = "macOS_5_0_0_beta50_rc3",
	Linux_5_0_0_beta50 = "Linux_5_0_0_beta50",
	macOS_5_0_0_beta50 = "macOS_5_0_0_beta50",
	Windows_5_0_0_beta50 = "Windows_5_0_0_beta50",
	Windows_5_0_0_qa_51_1 = "Windows_5_0_0_qa_51_1",
	Windows_5_0_0_beta51_rc1 = "Windows_5_0_0_beta51_rc1",
	Windows_5_0_0_beta51_rc2 = "Windows_5_0_0_beta51_rc2",
	Windows_5_0_0_beta51_rc4__1 = "Windows_5_0_0_beta51_rc4__1",
	Windows_5_0_0_beta51_rc4__2 = "Windows_5_0_0_beta51_rc4__2",
	Windows_5_0_0_beta51_rc5 = "Windows_5_0_0_beta51_rc5",
	Linux_5_0_0_beta51 = "Linux_5_0_0_beta51",
	macOS_5_0_0_beta51 = "macOS_5_0_0_beta51",
	Windows_5_0_0_beta51 = "Windows_5_0_0_beta51",
	Windows_5_0_0_qa_52_2 = "Windows_5_0_0_qa_52_2",
	Windows_5_0_0_qa_52_4 = "Windows_5_0_0_qa_52_4",
	Windows_5_0_0_beta52_rc1 = "Windows_5_0_0_beta52_rc1",
	Windows_5_0_0_qa_52_6 = "Windows_5_0_0_qa_52_6",
	Windows_5_0_0_beta52_rc2 = "Windows_5_0_0_beta52_rc2",
	Linux_5_0_0_beta52 = "Linux_5_0_0_beta52",
	macOS_5_0_0_beta52 = "macOS_5_0_0_beta52",
	Windows_5_0_0_beta52 = "Windows_5_0_0_beta52",
	Linux_5_0_0_beta52_1 = "Linux_5_0_0_beta52_1",
	macOS_5_0_0_beta52_1 = "macOS_5_0_0_beta52_1",
	Windows_5_0_0_beta52_1 = "Windows_5_0_0_beta52_1",
	Windows_5_0_0_beta53_rc1 = "Windows_5_0_0_beta53_rc1",
	Windows_5_0_0_qa_53_2 = "Windows_5_0_0_qa_53_2",
	Windows_5_0_0_qa_53_3 = "Windows_5_0_0_qa_53_3",
	Windows_5_0_0_beta53_rc3 = "Windows_5_0_0_beta53_rc3",
	Linux_5_0_0_beta53 = "Linux_5_0_0_beta53",
	macOS_5_0_0_beta53 = "macOS_5_0_0_beta53",
	Windows_5_0_0_beta53 = "Windows_5_0_0_beta53",
	Windows_5_0_0_qa_th_7 = "Windows_5_0_0_qa_th_7",
	Windows_5_0_0_beta54_rc1 = "Windows_5_0_0_beta54_rc1",
	Windows_5_0_0_qa_sea_3 = "Windows_5_0_0_qa_sea_3",
	Windows_5_0_0_beta54_rc2 = "Windows_5_0_0_beta54_rc2",
	Windows_5_0_0_qa_ex_1 = "Windows_5_0_0_qa_ex_1",
	macOS_5_0_0_beta54_rc3 = "macOS_5_0_0_beta54_rc3",
	Windows_5_0_0_beta54_rc3 = "Windows_5_0_0_beta54_rc3",
	Linux_5_0_0_beta54 = "Linux_5_0_0_beta54",
	macOS_5_0_0_beta54 = "macOS_5_0_0_beta54",
	Windows_5_0_0_beta54 = "Windows_5_0_0_beta54",
	Windows_5_0_0_qa_55_1 = "Windows_5_0_0_qa_55_1",
	Linux_5_0_0_beta54_1 = "Linux_5_0_0_beta54_1",
	macOS_5_0_0_beta54_1 = "macOS_5_0_0_beta54_1",
	Windows_5_0_0_beta54_1 = "Windows_5_0_0_beta54_1",
	Windows_5_0_0_beta55_rc1 = "Windows_5_0_0_beta55_rc1",
	Windows_5_0_0_beta55_rc3 = "Windows_5_0_0_beta55_rc3",
	Linux_5_0_0_beta55 = "Linux_5_0_0_beta55",
	macOS_5_0_0_beta55 = "macOS_5_0_0_beta55",
	Windows_5_0_0_beta55 = "Windows_5_0_0_beta55",
	Windows_5_0_0_qa_56_1 = "Windows_5_0_0_qa_56_1",
	Linux_5_0_0_beta56 = "Linux_5_0_0_beta56",
	macOS_5_0_0_beta56 = "macOS_5_0_0_beta56",
	Windows_5_0_0_beta56 = "Windows_5_0_0_beta56",
	Linux_5_0_0_beta57_rc1 = "Linux_5_0_0_beta57_rc1",
	Windows_5_0_0_beta57_rc1 = "Windows_5_0_0_beta57_rc1",
	Linux_5_0_0_beta57_rc2 = "Linux_5_0_0_beta57_rc2",
	macOS_5_0_0_beta57_rc2 = "macOS_5_0_0_beta57_rc2",
	Windows_5_0_0_beta57_rc2 = "Windows_5_0_0_beta57_rc2",
	Windows_5_0_0_qa_57_2 = "Windows_5_0_0_qa_57_2",
	Windows_5_0_0_qa_57_3_1 = "Windows_5_0_0_qa_57_3_1",
	Linux_5_0_0_beta57 = "Linux_5_0_0_beta57",
	macOS_5_0_0_beta57 = "macOS_5_0_0_beta57",
	Windows_5_0_0_beta57 = "Windows_5_0_0_beta57",
	macOS_5_0_0_beta58_rc1 = "macOS_5_0_0_beta58_rc1",
	Windows_5_0_0_qa_ex_3 = "Windows_5_0_0_qa_ex_3",
	Linux_5_0_0_beta58_rc2 = "Linux_5_0_0_beta58_rc2",
	macOS_5_0_0_beta58_rc2 = "macOS_5_0_0_beta58_rc2",
	Windows_5_0_0_beta58_rc2 = "Windows_5_0_0_beta58_rc2",
	Linux_5_0_0_beta58_rc3 = "Linux_5_0_0_beta58_rc3",
	Windows_5_0_0_beta58_rc3 = "Windows_5_0_0_beta58_rc3",
	Linux_5_0_0_beta58_rc4 = "Linux_5_0_0_beta58_rc4",
	Windows_5_0_0_beta58_rc4 = "Windows_5_0_0_beta58_rc4",
	Linux_5_0_0_beta58_rc5 = "Linux_5_0_0_beta58_rc5",
	Windows_5_0_0_beta58_rc5 = "Windows_5_0_0_beta58_rc5",
	Linux_5_0_0_beta58_rc6 = "Linux_5_0_0_beta58_rc6",
	Windows_5_0_0_beta58_rc6 = "Windows_5_0_0_beta58_rc6",
	Linux_5_0_0_beta58_rc7 = "Linux_5_0_0_beta58_rc7",
	Windows_5_0_0_beta58_rc7 = "Windows_5_0_0_beta58_rc7",
	Linux_5_0_0_beta58 = "Linux_5_0_0_beta58",
	macOS_5_0_0_beta58 = "macOS_5_0_0_beta58",
	Windows_5_0_0_beta58 = "Windows_5_0_0_beta58",
	Linux_5_0_0_beta58_1 = "Linux_5_0_0_beta58_1",
	macOS_5_0_0_beta58_1 = "macOS_5_0_0_beta58_1",
	Windows_5_0_0_beta58_1 = "Windows_5_0_0_beta58_1",
	Linux_5_0_0_beta59_rc3 = "Linux_5_0_0_beta59_rc3",
	Windows_5_0_0_beta59_rc3 = "Windows_5_0_0_beta59_rc3",
	Linux_5_0_0_beta59_rc5 = "Linux_5_0_0_beta59_rc5",
	Windows_5_0_0_beta59_rc5 = "Windows_5_0_0_beta59_rc5",
	Linux_5_0_0_beta59_rc6 = "Linux_5_0_0_beta59_rc6",
	Windows_5_0_0_beta59_rc6 = "Windows_5_0_0_beta59_rc6",
	Linux_5_0_0_beta59 = "Linux_5_0_0_beta59",
	macOS_5_0_0_beta59 = "macOS_5_0_0_beta59",
	Windows_5_0_0_beta59 = "Windows_5_0_0_beta59",
	Windows_5_0_0_qa_60_0 = "Windows_5_0_0_qa_60_0",
	Windows_5_0_0_qa_60_5 = "Windows_5_0_0_qa_60_5",
	Linux_5_0_0_beta60_rc1 = "Linux_5_0_0_beta60_rc1",
	Windows_5_0_0_beta60_rc1 = "Windows_5_0_0_beta60_rc1",
	Windows_5_0_0_beta60_rc2 = "Windows_5_0_0_beta60_rc2",
	Windows_5_0_0_beta60_rc3 = "Windows_5_0_0_beta60_rc3",
	Windows_5_0_0_beta60_rc4 = "Windows_5_0_0_beta60_rc4",
	Linux_5_0_0_beta60 = "Linux_5_0_0_beta60",
	macOS_5_0_0_beta60 = "macOS_5_0_0_beta60",
	Windows_5_0_0_beta60 = "Windows_5_0_0_beta60",
	Linux_5_0_0_beta60_2__1 = "Linux_5_0_0_beta60_2__1",
	macOS_5_0_0_beta60_2__1 = "macOS_5_0_0_beta60_2__1",
	Windows_5_0_0_beta60_1 = "Windows_5_0_0_beta60_1",
	Linux_5_0_0_beta60_2__2 = "Linux_5_0_0_beta60_2__2",
	macOS_5_0_0_beta60_2__2 = "macOS_5_0_0_beta60_2__2",
	Windows_5_0_0_beta60_2 = "Windows_5_0_0_beta60_2",
	Linux_5_0_0_beta61_rc1 = "Linux_5_0_0_beta61_rc1",
	Windows_5_0_0_beta61_rc1 = "Windows_5_0_0_beta61_rc1",
	Linux_5_0_0_beta61_rc2 = "Linux_5_0_0_beta61_rc2",
	Windows_5_0_0_beta61_rc2 = "Windows_5_0_0_beta61_rc2",
	Linux_5_0_0_beta61_rc3 = "Linux_5_0_0_beta61_rc3",
	macOS_5_0_0_beta61_rc3 = "macOS_5_0_0_beta61_rc3",
	Linux_5_0_0_beta61 = "Linux_5_0_0_beta61",
	macOS_5_0_0_beta61 = "macOS_5_0_0_beta61",
	Windows_5_0_0_beta61 = "Windows_5_0_0_beta61",
	Linux_5_0_0_beta62_rc1 = "Linux_5_0_0_beta62_rc1",
	Windows_5_0_0_qa_1925_8 = "Windows_5_0_0_qa_1925_8",
	Linux_5_0_0_beta62_rc2 = "Linux_5_0_0_beta62_rc2",
	Windows_5_0_0_beta62_rc2 = "Windows_5_0_0_beta62_rc2",
	macOS_5_0_0_beta61_1 = "macOS_5_0_0_beta61_1",
	Windows_5_0_0_beta61_1 = "Windows_5_0_0_beta61_1",
	Linux_5_0_0_beta62_rc4 = "Linux_5_0_0_beta62_rc4",
	Windows_5_0_0_beta62_rc4 = "Windows_5_0_0_beta62_rc4",
	Windows_5_0_0_beta62_rc5 = "Windows_5_0_0_beta62_rc5",
	Windows_5_0_0_qa_1925_10 = "Windows_5_0_0_qa_1925_10",
	Linux_5_0_0_beta62 = "Linux_5_0_0_beta62",
	macOS_5_0_0_beta62 = "macOS_5_0_0_beta62",
	Windows_5_0_0_beta62 = "Windows_5_0_0_beta62",
	Linux_5_0_0_beta63_rc1 = "Linux_5_0_0_beta63_rc1",
	Windows_5_0_0_beta63_rc1 = "Windows_5_0_0_beta63_rc1",
	Linux_5_0_0_beta63_rc2 = "Linux_5_0_0_beta63_rc2",
	Windows_5_0_0_beta63_rc2 = "Windows_5_0_0_beta63_rc2",
	Windows_5_0_0_beta63_rc3 = "Windows_5_0_0_beta63_rc3",
	Windows_5_0_0_beta63_rc4 = "Windows_5_0_0_beta63_rc4",
	Linux_5_0_0_beta63_rc5 = "Linux_5_0_0_beta63_rc5",
	Windows_5_0_0_beta63_rc5 = "Windows_5_0_0_beta63_rc5",
	Linux_5_0_0_beta63_rc6 = "Linux_5_0_0_beta63_rc6",
	Windows_5_0_0_beta63_rc6 = "Windows_5_0_0_beta63_rc6",
	Windows_5_0_0_beta63_rc8 = "Windows_5_0_0_beta63_rc8",
	Linux_5_0_0_beta63_rc9 = "Linux_5_0_0_beta63_rc9",
	Windows_5_0_0_beta63_rc9 = "Windows_5_0_0_beta63_rc9",
	Linux_5_0_0_beta63 = "Linux_5_0_0_beta63",
	macOS_5_0_0_beta63 = "macOS_5_0_0_beta63",
	Windows_5_0_0_beta63 = "Windows_5_0_0_beta63",
	Windows_5_0_0_away_test8 = "Windows_5_0_0_away_test8",
	Linux_5_0_0_beta63_1 = "Linux_5_0_0_beta63_1",
	macOS_5_0_0_beta63_1 = "macOS_5_0_0_beta63_1",
	Windows_5_0_0_beta63_1 = "Windows_5_0_0_beta63_1",
	Windows_5_0_0_beta64_rc1 = "Windows_5_0_0_beta64_rc1",
	Windows_5_0_0_beta64_rc2 = "Windows_5_0_0_beta64_rc2",
	Linux_5_0_0_beta64_rc3 = "Linux_5_0_0_beta64_rc3",
	Windows_5_0_0_beta64_rc3 = "Windows_5_0_0_beta64_rc3",
	macOS_5_0_0_beta64 = "macOS_5_0_0_beta64",
	Windows_5_0_0_beta64 = "Windows_5_0_0_beta64",
	Windows_5_0_0_qa_66_1 = "Windows_5_0_0_qa_66_1",
	Linux_5_0_0_beta65 = "Linux_5_0_0_beta65",
	Windows_5_0_0_beta65 = "Windows_5_0_0_beta65",
	Windows_5_0_0_beta65_1_rc1 = "Windows_5_0_0_beta65_1_rc1",
	Linux_5_0_0_beta65_1 = "Linux_5_0_0_beta65_1",
	macOS_5_0_0_beta65_1 = "macOS_5_0_0_beta65_1",
	Windows_5_0_0_beta65_1 = "Windows_5_0_0_beta65_1",
	Windows_5_0_0_qa_66_4 = "Windows_5_0_0_qa_66_4",
	Windows_5_0_0_ms_2 = "Windows_5_0_0_ms_2",
	Windows_5_0_0_qa_66_6 = "Windows_5_0_0_qa_66_6",
	Windows_5_0_0_beta66_internal = "Windows_5_0_0_beta66_internal",
	Linux_5_0_0_beta66_rc1 = "Linux_5_0_0_beta66_rc1",
	Windows_5_0_0_beta66_rc1 = "Windows_5_0_0_beta66_rc1",
	Linux_5_0_0_beta66 = "Linux_5_0_0_beta66",
	macOS_5_0_0_beta66 = "macOS_5_0_0_beta66",
	Windows_5_0_0_beta66 = "Windows_5_0_0_beta66",
	Linux_5_0_0_beta67_rc1 = "Linux_5_0_0_beta67_rc1",
	Windows_5_0_0_beta67_rc1 = "Windows_5_0_0_beta67_rc1",
	Linux_5_0_0_beta67_rc2 = "Linux_5_0_0_beta67_rc2",
	macOS_5_0_0_beta67_rc2 = "macOS_5_0_0_beta67_rc2",
	Windows_5_0_0_beta67_rc2 = "Windows_5_0_0_beta67_rc2",
	Linux_5_0_0_beta67_rc3 = "Linux_5_0_0_beta67_rc3",
	Windows_5_0_0_beta67_rc3 = "Windows_5_0_0_beta67_rc3",
	Linux_5_0_0_beta67_rc4 = "Linux_5_0_0_beta67_rc4",
	Windows_5_0_0_beta67_rc4 = "Windows_5_0_0_beta67_rc4",
	Linux_5_0_0_beta67_rc5 = "Linux_5_0_0_beta67_rc5",
	Windows_5_0_0_beta67_rc5 = "Windows_5_0_0_beta67_rc5",
	Windows_5_0_0_olm_refactor_1 = "Windows_5_0_0_olm_refactor_1",
	Linux_5_0_0_beta67 = "Linux_5_0_0_beta67",
	macOS_5_0_0_beta67 = "macOS_5_0_0_beta67",
	Windows_5_0_0_beta67 = "Windows_5_0_0_beta67",
	macOS_5_0_0_qa_68_2 = "macOS_5_0_0_qa_68_2",
	Windows_5_0_0_qa_store_update_dummy_fix_1 = "Windows_5_0_0_qa_store_update_dummy_fix_1",
	Windows_5_0_0_qa_store_update_dummy_fix_2 = "Windows_5_0_0_qa_store_update_dummy_fix_2",
	Windows_5_0_0_qa_th_16 = "Windows_5_0_0_qa_th_16",
	iOS_3_6_0 = "iOS_3_6_0",
	Windows_5_0_0_qa_68_7 = "Windows_5_0_0_qa_68_7",
	Windows_5_0_0_qa_68_9 = "Windows_5_0_0_qa_68_9",
	Windows_5_0_0_qa_68_10 = "Windows_5_0_0_qa_68_10",
	Windows_5_0_0_qa_68_11 = "Windows_5_0_0_qa_68_11",
	Linux_5_0_0_beta68_rc1 = "Linux_5_0_0_beta68_rc1",
	Windows_5_0_0_beta68_rc1 = "Windows_5_0_0_beta68_rc1",
	Linux_5_0_0_beta68_rc2 = "Linux_5_0_0_beta68_rc2",
	Windows_5_0_0_beta68_rc2 = "Windows_5_0_0_beta68_rc2",
	Windows_3_6_0__1 = "Windows_3_6_0__1",
	Windows_5_0_0_qa_68_20 = "Windows_5_0_0_qa_68_20",
	Linux_5_0_0_beta68_rc3 = "Linux_5_0_0_beta68_rc3",
	Windows_5_0_0_beta68_rc3 = "Windows_5_0_0_beta68_rc3",
	Windows_5_0_0_qa_68_26 = "Windows_5_0_0_qa_68_26",
	Windows_5_0_0_qa_68_27 = "Windows_5_0_0_qa_68_27",
	macOS_5_0_0_beta68 = "macOS_5_0_0_beta68",
	Windows_5_0_0_beta68 = "Windows_5_0_0_beta68",
	Windows_5_0_0_beta68_1__1 = "Windows_5_0_0_beta68_1__1",
	Windows_5_0_0_qa_69_5 = "Windows_5_0_0_qa_69_5",
	Windows_5_0_0_qa_69_6 = "Windows_5_0_0_qa_69_6",
	Linux_5_0_0_beta68_1 = "Linux_5_0_0_beta68_1",
	macOS_5_0_0_beta68_1 = "macOS_5_0_0_beta68_1",
	Windows_5_0_0_beta68_1__2 = "Windows_5_0_0_beta68_1__2",
	Windows_5_0_0_beta69_rc1 = "Windows_5_0_0_beta69_rc1",
	Linux_5_0_0_beta69_rc2 = "Linux_5_0_0_beta69_rc2",
	Windows_5_0_0_beta69_rc2 = "Windows_5_0_0_beta69_rc2",
	Linux_5_0_0_beta69_rc3 = "Linux_5_0_0_beta69_rc3",
	Windows_5_0_0_beta69_rc3 = "Windows_5_0_0_beta69_rc3",
	Linux_5_0_0_beta69_rc4 = "Linux_5_0_0_beta69_rc4",
	Windows_5_0_0_beta69_rc4 = "Windows_5_0_0_beta69_rc4",
	Android_5_0_0 = "Android_5_0_0",
	Windows_5_0_0_beta69_rc5 = "Windows_5_0_0_beta69_rc5",
	Linux_5_0_0_beta69_rc6 = "Linux_5_0_0_beta69_rc6",
	Windows_5_0_0_beta69_rc6 = "Windows_5_0_0_beta69_rc6",
	Windows_5_0_0_qa_69_12 = "Windows_5_0_0_qa_69_12",
	Windows_5_0_0_qa_69_14 = "Windows_5_0_0_qa_69_14",
	Windows_5_0_0_qa_69_15 = "Windows_5_0_0_qa_69_15",
	Windows_5_0_0_qa_69_16 = "Windows_5_0_0_qa_69_16",
	Windows_5_0_0_qa_69_18 = "Windows_5_0_0_qa_69_18",
	Windows_5_0_0_qa_69_20 = "Windows_5_0_0_qa_69_20",
	Windows_5_0_0_qa_request_chat_7 = "Windows_5_0_0_qa_request_chat_7",
	Windows_5_0_0_qa_69_21 = "Windows_5_0_0_qa_69_21",
	Windows_5_0_0_t_2915_2 = "Windows_5_0_0_t_2915_2",
	Windows_3_6_0__2 = "Windows_3_6_0__2",
	Windows_3_6_0__3 = "Windows_3_6_0__3",
	Linux_3_6_0__1 = "Linux_3_6_0__1",
	Windows_3_6_0__4 = "Windows_3_6_0__4",
	Linux_3_6_0__2 = "Linux_3_6_0__2",
	Windows_3_6_0__5 = "Windows_3_6_0__5",
	Windows_3_6_0__6 = "Windows_3_6_0__6",
	Windows_3_6_0__7 = "Windows_3_6_0__7",
	Windows_3_6_0__8 = "Windows_3_6_0__8",
	macOS_3_6_0__1 = "macOS_3_6_0__1",
	Windows_3_6_0__9 = "Windows_3_6_0__9",
	Windows_5_0_0_qa_69_26 = "Windows_5_0_0_qa_69_26",
	Linux_3_6_0__3 = "Linux_3_6_0__3",
	macOS_3_6_0__2 = "macOS_3_6_0__2",
	Windows_3_6_0__10 = "Windows_3_6_0__10",
	Windows_5_0_0_qa_69_27 = "Windows_5_0_0_qa_69_27",
	Windows_5_0_0_qa_69_28 = "Windows_5_0_0_qa_69_28",
	Windows_5_0_0_qa_69_29 = "Windows_5_0_0_qa_69_29",
	Windows_5_0_0_qa_69_30 = "Windows_5_0_0_qa_69_30",
	Windows_5_0_0_beta69_rc8 = "Windows_5_0_0_beta69_rc8",
	Linux_5_0_0_beta69_rc8 = "Linux_5_0_0_beta69_rc8",
	macOS_5_0_0_beta69_rc8 = "macOS_5_0_0_beta69_rc8",
	Linux_5_0_0_beta69__1 = "Linux_5_0_0_beta69__1",
	Windows_5_0_0_beta69__1 = "Windows_5_0_0_beta69__1",
	Linux_5_0_0_beta69__2 = "Linux_5_0_0_beta69__2",
	macOS_5_0_0_beta69 = "macOS_5_0_0_beta69",
	Windows_5_0_0_beta69__2 = "Windows_5_0_0_beta69__2",
	Windows_5_0_0_qa_69_34 = "Windows_5_0_0_qa_69_34",
	Linux_5_0_0_beta70_rc2 = "Linux_5_0_0_beta70_rc2",
	Windows_5_0_0_t_2915_7 = "Windows_5_0_0_t_2915_7",
	Windows_5_0_0_qa_70_1 = "Windows_5_0_0_qa_70_1",
	Linux_5_0_0_beta70_rc3 = "Linux_5_0_0_beta70_rc3",
	Windows_5_0_0_beta70_rc3 = "Windows_5_0_0_beta70_rc3",
	Linux_5_0_0_beta70_rc4 = "Linux_5_0_0_beta70_rc4",
	Windows_5_0_0_beta70_rc4 = "Windows_5_0_0_beta70_rc4",
	Windows_5_0_0_qa_70_2 = "Windows_5_0_0_qa_70_2",
	Windows_5_0_0_beta70_rc5 = "Windows_5_0_0_beta70_rc5",
	Linux_5_0_0_beta70_rc6 = "Linux_5_0_0_beta70_rc6",
	Windows_5_0_0_beta70_rc6 = "Windows_5_0_0_beta70_rc6",
	Windows_3_6_0__11 = "Windows_3_6_0__11",
	Linux_5_0_0_beta70_rc7 = "Linux_5_0_0_beta70_rc7",
	Windows_5_0_0_beta70_rc7 = "Windows_5_0_0_beta70_rc7",
	macOS_3_6_0__3 = "macOS_3_6_0__3",
	Windows_3_6_0__12 = "Windows_3_6_0__12",
	Linux_5_0_0_beta70 = "Linux_5_0_0_beta70",
	macOS_5_0_0_beta70 = "macOS_5_0_0_beta70",
	Windows_5_0_0_beta70 = "Windows_5_0_0_beta70",
	Linux_5_0_0_qa_2913_2 = "Linux_5_0_0_qa_2913_2",
	Windows_5_0_0_qa_2913_2 = "Windows_5_0_0_qa_2913_2",
	Windows_5_0_0_qa_2913_3 = "Windows_5_0_0_qa_2913_3",
	Windows_5_0_0_beta71_rc1 = "Windows_5_0_0_beta71_rc1",
	Linux_5_0_0_beta71_rc2 = "Linux_5_0_0_beta71_rc2",
	Windows_5_0_0_beta71_rc2 = "Windows_5_0_0_beta71_rc2",
	Windows_5_0_0_qa_71_1 = "Windows_5_0_0_qa_71_1",
	Windows_5_0_0_qa_71_2 = "Windows_5_0_0_qa_71_2",
	Windows_5_0_0_beta71_rc3 = "Windows_5_0_0_beta71_rc3",
	Linux_5_0_0_beta71_rc4 = "Linux_5_0_0_beta71_rc4",
	Windows_5_0_0_beta71_rc4 = "Windows_5_0_0_beta71_rc4",
	Windows_5_0_0_qa_71_3 = "Windows_5_0_0_qa_71_3",
	Linux_5_0_0_beta71_rc6 = "Linux_5_0_0_beta71_rc6",
	Windows_5_0_0_beta71_rc6 = "Windows_5_0_0_beta71_rc6",
	Windows_5_0_0_tra_09 = "Windows_5_0_0_tra_09",
	Linux_5_0_0_beta71_rc7 = "Linux_5_0_0_beta71_rc7",
	macOS_5_0_0_beta71_rc7 = "macOS_5_0_0_beta71_rc7",
	Windows_5_0_0_qa_71_4 = "Windows_5_0_0_qa_71_4",
	Windows_5_0_0_qa_71_5 = "Windows_5_0_0_qa_71_5",
	Windows_5_0_0_qa_71_6 = "Windows_5_0_0_qa_71_6",
	Windows_5_0_0_qa_71_7 = "Windows_5_0_0_qa_71_7",
	Windows_5_0_0_beta71_rc9 = "Windows_5_0_0_beta71_rc9",
	Windows_5_0_0_qa_3231_7 = "Windows_5_0_0_qa_3231_7",
	Windows_5_0_0_beta71_rc12 = "Windows_5_0_0_beta71_rc12",
	Linux_5_0_0_beta71_rc13 = "Linux_5_0_0_beta71_rc13",
	macOS_5_0_0_beta71_rc13 = "macOS_5_0_0_beta71_rc13",
	Windows_5_0_0_beta71_rc13 = "Windows_5_0_0_beta71_rc13",
	Linux_3_6_0__4 = "Linux_3_6_0__4",
	Windows_3_6_0__13 = "Windows_3_6_0__13",
	Windows3__6_0 = "Windows3__6_0",
	Windows_5_0_0_qa_3305_rc13 = "Windows_5_0_0_qa_3305_rc13",
	Windows_5_0_0_qa_71_8 = "Windows_5_0_0_qa_71_8",
	Linux_5_0_0_beta71_rc14 = "Linux_5_0_0_beta71_rc14",
	Windows_5_0_0_beta71_rc14 = "Windows_5_0_0_beta71_rc14",
	Windows_5_0_0_qa_pre_crypto_fixes = "Windows_5_0_0_qa_pre_crypto_fixes",
	Windows_5_0_0_qa_crypto_fixes_1 = "Windows_5_0_0_qa_crypto_fixes_1",
	Windows_5_0_0_qa_crypto_test_on_rc14_2 = "Windows_5_0_0_qa_crypto_test_on_rc14_2",
	Windows_5_0_0_ui_polishing_11 = "Windows_5_0_0_ui_polishing_11",
	Windows_5_0_0_qa_crypto_fixes_3 = "Windows_5_0_0_qa_crypto_fixes_3",
	Windows_5_0_0_qa_71_9 = "Windows_5_0_0_qa_71_9",
	Windows_5_0_0_qa_71_10 = "Windows_5_0_0_qa_71_10",
	Windows_5_0_0_qa_71_12 = "Windows_5_0_0_qa_71_12",
	Linux_5_0_0_beta71_rc15 = "Linux_5_0_0_beta71_rc15",
	Windows_5_0_0_beta71_rc15 = "Windows_5_0_0_beta71_rc15",
	Linux_3_6_0__5 = "Linux_3_6_0__5",
	macOS_3_6_0__4 = "macOS_3_6_0__4",
	Windows_3_6_0__14 = "Windows_3_6_0__14",
	Windows_5_0_0_qa_3230_1 = "Windows_5_0_0_qa_3230_1",
	Windows_5_0_0_qa_71_15 = "Windows_5_0_0_qa_71_15",
	Windows_5_0_0_qa_3230_2 = "Windows_5_0_0_qa_3230_2",
	Android_3_6_0 = "Android_3_6_0",
	Windows_5_0_0_qa_71_17 = "Windows_5_0_0_qa_71_17",
	Windows_5_0_0_qa_71_18 = "Windows_5_0_0_qa_71_18",
	Windows_5_0_0_qa_71_19 = "Windows_5_0_0_qa_71_19",
	Windows_5_0_0_qa_71_21 = "Windows_5_0_0_qa_71_21",
	Windows_5_0_0_qa_71_24 = "Windows_5_0_0_qa_71_24",
	Linux_5_0_0_beta71_rc16 = "Linux_5_0_0_beta71_rc16",
	Windows_5_0_0_beta71_rc16 = "Windows_5_0_0_beta71_rc16",
	Windows_5_0_0_qa_71_26 = "Windows_5_0_0_qa_71_26",
	Windows_5_0_0_qa_71_29 = "Windows_5_0_0_qa_71_29",
	Linux_5_0_0_beta71 = "Linux_5_0_0_beta71",
	macOS_5_0_0_beta71 = "macOS_5_0_0_beta71",
	Windows_5_0_0_beta71 = "Windows_5_0_0_beta71",
	Linux_5_0_0_beta72 = "Linux_5_0_0_beta72",
	Windows_5_0_0_beta72_rc1 = "Windows_5_0_0_beta72_rc1",
	Windows_5_0_0_beta72 = "Windows_5_0_0_beta72",
	Windows_5_0_0_qa_73_3 = "Windows_5_0_0_qa_73_3",
	Windows_5_0_0_qa_73_11 = "Windows_5_0_0_qa_73_11",
	Linux_5_0_0_beta73_rc1 = "Linux_5_0_0_beta73_rc1",
	Windows_5_0_0_beta73_rc1 = "Windows_5_0_0_beta73_rc1",
	Linux_5_0_0_beta73_rc3 = "Linux_5_0_0_beta73_rc3",
	Windows_5_0_0_beta73_rc3 = "Windows_5_0_0_beta73_rc3",
	Linux_5_0_0_beta73 = "Linux_5_0_0_beta73",
	OS_X_5_0_0_beta73 = "OS_X_5_0_0_beta73",
	Windows_5_0_0_beta73 = "Windows_5_0_0_beta73",
	Windows_5_0_0_qa_74_1 = "Windows_5_0_0_qa_74_1",
	Windows_5_0_0_qa_74_2 = "Windows_5_0_0_qa_74_2",
	Windows_5_0_0_qa_74_3 = "Windows_5_0_0_qa_74_3",
	Android_3_X_X = "Android_3_X_X",
	iOS_3_X_X = "iOS_3_X_X",
	Linux_3_X_X = "Linux_3_X_X",
	OS_X_3_X_X = "OS_X_3_X_X",
	Windows_3_X_X__1 = "Windows_3_X_X__1",
	Windows_3_X_X__2 = "Windows_3_X_X__2",
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

export class ServerGroupGen extends ServerGroupBase {
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

	public override update(obj: Partial<this> | Partial<ServerGroupGen>): this {
		Object.assign(this, obj);
		return this;
	}
}

export class ChannelGroupGen extends ChannelGroupBase {
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

	public override update(obj: Partial<this> | Partial<ChannelGroupGen>): this {
		Object.assign(this, obj);
		return this;
	}
}

export class OptionalChannelDataGen extends OptionalChannelDataBase {
	public readonly description!: string;
	public readonly descriptionRendered!: string;

	public override update(obj: Partial<this> | Partial<OptionalChannelDataGen>): this {
		Object.assign(this, obj);
		return this;
	}

	public static fromJson(obj: Partial<OptionalChannelDataGen>): OptionalChannelDataGen {
		return new OptionalChannelDataGen().update(obj);
	}
}

export class ChannelGen extends ChannelBase {
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
	public readonly storageQuota!: number | null;
	public readonly subscribed!: boolean;
	public readonly permissionHints!: ChannelPermissionHint | null;
	public readonly optionalData!: OptionalChannelDataGen | null;

	public override update(obj: Partial<this> | Partial<ChannelGen>): this {
		if (obj.deleteDelay)
			(obj as any).deleteDelay = durationDeserialize((obj as any).deleteDelay);
		if (obj.optionalData !== undefined && obj.optionalData !== null)
			(obj as any).optionalData = OptionalChannelDataGen.fromJson(obj.optionalData!);
		Object.assign(this, obj);
		return this;
	}
}

export class OptionalClientDataGen extends OptionalClientDataBase {
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

	public override update(obj: Partial<this> | Partial<OptionalClientDataGen>): this {
		if (obj.created)
			(obj as any).created = datetimeDeserialize((obj as any).created);
		if (obj.lastConnected)
			(obj as any).lastConnected = datetimeDeserialize((obj as any).lastConnected);
		Object.assign(this, obj);
		return this;
	}

	public static fromJson(obj: Partial<OptionalClientDataGen>): OptionalClientDataGen {
		return new OptionalClientDataGen().update(obj);
	}
}

export class ConnectionClientDataGen extends ConnectionClientDataBase {
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

	public override update(obj: Partial<this> | Partial<ConnectionClientDataGen>): this {
		if (obj.ping)
			(obj as any).ping = durationDeserialize((obj as any).ping);
		if (obj.pingDeviation)
			(obj as any).pingDeviation = durationDeserialize((obj as any).pingDeviation);
		if (obj.connectedTime)
			(obj as any).connectedTime = durationDeserialize((obj as any).connectedTime);
		if (obj.idleTime)
			(obj as any).idleTime = durationDeserialize((obj as any).idleTime);
		Object.assign(this, obj);
		return this;
	}

	public static fromJson(obj: Partial<ConnectionClientDataGen>): ConnectionClientDataGen {
		return new ConnectionClientDataGen().update(obj);
	}
}

export class ClientGen extends ClientBase {
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
	public readonly userTag!: string | null;
	public readonly permissionHints!: ClientPermissionHint | null;
	public readonly optionalData!: OptionalClientDataGen | null;
	public readonly connectionData!: ConnectionClientDataGen | null;

	public override update(obj: Partial<this> | Partial<ClientGen>): this {
		if (obj.optionalData !== undefined && obj.optionalData !== null)
			(obj as any).optionalData = OptionalClientDataGen.fromJson(obj.optionalData!);
		if (obj.connectionData !== undefined && obj.connectionData !== null)
			(obj as any).connectionData = ConnectionClientDataGen.fromJson(obj.connectionData!);
		Object.assign(this, obj);
		return this;
	}
}

export class OptionalServerDataGen extends OptionalServerDataBase {
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

	public override update(obj: Partial<this> | Partial<OptionalServerDataGen>): this {
		if (obj.uptime)
			(obj as any).uptime = durationDeserialize((obj as any).uptime);
		if (obj.complainAutobanTime)
			(obj as any).complainAutobanTime = durationDeserialize((obj as any).complainAutobanTime);
		if (obj.complainRemoveTime)
			(obj as any).complainRemoveTime = durationDeserialize((obj as any).complainRemoveTime);
		if (obj.minClientVersion)
			(obj as any).minClientVersion = datetimeDeserialize((obj as any).minClientVersion);
		if (obj.totalPing)
			(obj as any).totalPing = durationDeserialize((obj as any).totalPing);
		if (obj.minAndroidVersion)
			(obj as any).minAndroidVersion = datetimeDeserialize((obj as any).minAndroidVersion);
		if (obj.minIosVersion)
			(obj as any).minIosVersion = datetimeDeserialize((obj as any).minIosVersion);
		Object.assign(this, obj);
		return this;
	}

	public static fromJson(obj: Partial<OptionalServerDataGen>): OptionalServerDataGen {
		return new OptionalServerDataGen().update(obj);
	}
}

export class ConnectionServerDataGen extends ConnectionServerDataBase {
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
	public readonly connectedTimeTotal!: Duration;
	public readonly packetlossTotal!: number;
	public readonly ping!: Duration;

	public override update(obj: Partial<this> | Partial<ConnectionServerDataGen>): this {
		if (obj.connectedTimeTotal)
			(obj as any).connectedTimeTotal = durationDeserialize((obj as any).connectedTimeTotal);
		if (obj.ping)
			(obj as any).ping = durationDeserialize((obj as any).ping);
		Object.assign(this, obj);
		return this;
	}

	public static fromJson(obj: Partial<ConnectionServerDataGen>): ConnectionServerDataGen {
		return new ConnectionServerDataGen().update(obj);
	}
}

export class ServerGen extends ServerBase {
	public readonly publicKey!: EccKeyPubP256;
	public readonly id!: string;
	public readonly name!: string;
	public readonly nickname!: string | null;
	public readonly welcomeMessage!: string;
	public readonly welcomeMessageRendered!: string;
	public readonly platform!: string;
	public readonly version!: string;
	public readonly maxClients!: number;
	public readonly created!: Moment;
	public readonly codecEncryptionMode!: CodecEncryptionMode;
	public readonly hostmessage!: string;
	public readonly hostmessageRendered!: string;
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
	public readonly administrativeDomain!: string | null;
	public readonly optionalData!: OptionalServerDataGen | null;
	public readonly connectionData!: ConnectionServerDataGen | null;

	public override update(obj: Partial<this> | Partial<ServerGen>): this {
		if (obj.created)
			(obj as any).created = datetimeDeserialize((obj as any).created);
		if (obj.hostbannerGfxInterval)
			(obj as any).hostbannerGfxInterval = durationDeserialize((obj as any).hostbannerGfxInterval);
		if (obj.tempChannelDefaultDeleteDelay)
			(obj as any).tempChannelDefaultDeleteDelay = durationDeserialize((obj as any).tempChannelDefaultDeleteDelay);
		if (obj.optionalData !== undefined && obj.optionalData !== null)
			(obj as any).optionalData = OptionalServerDataGen.fromJson(obj.optionalData!);
		if (obj.connectionData !== undefined && obj.connectionData !== null)
			(obj as any).connectionData = ConnectionServerDataGen.fromJson(obj.connectionData!);
		Object.assign(this, obj);
		return this;
	}
}

interface IMsgPropertyIdServerGroup {
	ServerGroup: ServerGroupId;
}

interface IMsgPropertyIdChannelGroup {
	ChannelGroup: ChannelGroupId;
}

interface IMsgPropertyIdOptionalChannelData {
	OptionalChannelData: ChannelId;
}

interface IMsgPropertyIdChannel {
	Channel: ChannelId;
}

interface IMsgPropertyIdOptionalClientData {
	OptionalClientData: ClientId;
}

interface IMsgPropertyIdConnectionClientData {
	ConnectionClientData: ClientId;
}

interface IMsgPropertyIdClient {
	Client: ClientId;
}

interface IMsgPropertyIdOptionalServerData {
	OptionalServerData: [];
}

interface IMsgPropertyIdConnectionServerData {
	ConnectionServerData: [];
}

interface IMsgPropertyIdServer {
	Server: [];
}

interface IMsgPropertyIdClientServerGroup {
	ClientServerGroup: [ClientId, ServerGroupId];
}

interface IMsgPropertyIdServerIp {
	ServerIp: IpAddr;
}

export type PropertyId =
	IMsgPropertyIdServerGroup
	| IMsgPropertyIdChannelGroup
	| IMsgPropertyIdOptionalChannelData
	| IMsgPropertyIdChannel
	| IMsgPropertyIdOptionalClientData
	| IMsgPropertyIdConnectionClientData
	| IMsgPropertyIdClient
	| IMsgPropertyIdOptionalServerData
	| IMsgPropertyIdConnectionServerData
	| IMsgPropertyIdServer
	| IMsgPropertyIdClientServerGroup
	| IMsgPropertyIdServerIp;

interface IMsgPropertyValueServerGroup {
	ServerGroup: Partial<ServerGroupGen>;
}

interface IMsgPropertyValueChannelGroup {
	ChannelGroup: Partial<ChannelGroupGen>;
}

interface IMsgPropertyValueOptionalChannelData {
	OptionalChannelData: Partial<OptionalChannelDataGen>;
}

interface IMsgPropertyValueChannel {
	Channel: Partial<ChannelGen>;
}

interface IMsgPropertyValueOptionalClientData {
	OptionalClientData: Partial<OptionalClientDataGen>;
}

interface IMsgPropertyValueConnectionClientData {
	ConnectionClientData: Partial<ConnectionClientDataGen>;
}

interface IMsgPropertyValueClient {
	Client: Partial<ClientGen>;
}

interface IMsgPropertyValueOptionalServerData {
	OptionalServerData: Partial<OptionalServerDataGen>;
}

interface IMsgPropertyValueConnectionServerData {
	ConnectionServerData: Partial<ConnectionServerDataGen>;
}

interface IMsgPropertyValueServer {
	Server: Partial<ServerGen>;
}

interface IMsgPropertyValueIpAddr {
	IpAddr: IpAddr;
}

interface IMsgPropertyValueServerGroupId {
	ServerGroupId: ServerGroupId;
}

export type PropertyValue =
	IMsgPropertyValueServerGroup
	| IMsgPropertyValueChannelGroup
	| IMsgPropertyValueOptionalChannelData
	| IMsgPropertyValueChannel
	| IMsgPropertyValueOptionalClientData
	| IMsgPropertyValueConnectionClientData
	| IMsgPropertyValueClient
	| IMsgPropertyValueOptionalServerData
	| IMsgPropertyValueConnectionServerData
	| IMsgPropertyValueServer
	| IMsgPropertyValueIpAddr
	| IMsgPropertyValueServerGroupId;

// Messages

export interface IMsgChannelListFinished {
	ChannelListFinished: Record<string, never>; // empty object
};

export interface IMsgChannelPasswordChanged {
	ChannelPasswordChanged: IMsgChannelPasswordChangedPart[];
}

export interface IMsgChannelPasswordChangedPart {
	channelId: ChannelId;
}

export interface IMsgChannelDescriptionChanged {
	ChannelDescriptionChanged: IMsgChannelDescriptionChangedPart[];
}

export interface IMsgChannelDescriptionChangedPart {
	channelId: ChannelId;
}

export interface IMsgChannelPermList {
	ChannelPermList: IMsgChannelPermListPart[];
}

export interface IMsgChannelPermListPart {
	channelId: ChannelId;
	permissionId: Permission;
	permissionValue: number;
	permissionNegated: boolean;
	permissionSkip: boolean;
}

export interface IMsgClientChatClosed {
	ClientChatClosed: IMsgClientChatClosedPart[];
}

export interface IMsgClientChatClosedPart {
	clientId: ClientId;
	clientUid: Uid;
}

export interface IMsgClientChatComposing {
	ClientChatComposing: IMsgClientChatComposingPart[];
}

export interface IMsgClientChatComposingPart {
	clientId: ClientId;
	clientUid: Uid;
}

export interface IMsgFiletransferStatus {
	FiletransferStatus: IMsgFiletransferStatusPart[];
}

export interface IMsgFiletransferStatusPart {
	clientFiletransferId: number;
	status: Error;
	message: string;
	size: string;
}

export interface IMsgPluginCommand {
	PluginCommand: IMsgPluginCommandPart[];
}

export interface IMsgPluginCommandPart {
	name: string;
	data: string;
	invokerId: ClientId | undefined;
	invokerName: string | undefined;
	invokerUid: Uid | undefined;
}

export interface IMsgFileInfo {
	FileInfo: IMsgFileInfoPart[];
}

export interface IMsgFileInfoPart {
	channelId: ChannelId;
	path: string;
	name: string;
	size: string;
	dateTime: OffsetDateTime;
}

export interface IMsgFileList {
	FileList: IMsgFileListPart[];
}

export interface IMsgFileListPart {
	channelId: ChannelId;
	path: string;
	name: string;
	size: string;
	dateTime: OffsetDateTime;
	isFile: boolean;
}

export interface IMsgFiletransfer {
	Filetransfer: IMsgFiletransferPart[];
}

export interface IMsgFiletransferPart {
	clientId: ClientId;
	path: string;
	name: string;
	size: string;
	sizeDone: number;
	clientFiletransferId: number;
	serverFiletransferId: number;
	sender: string;
	status: number;
	currentSpeed: number;
	averageSpeed: number;
	runtime: RustDuration;
}

export interface IMsgOfflineMessage {
	OfflineMessage: IMsgOfflineMessagePart[];
}

export interface IMsgOfflineMessagePart {
	messageId: number;
	clientUid: Uid;
	subject: string;
	subjectRendered: string;
	message: string;
	messageRendered: string;
	timestamp: OffsetDateTime;
}

export interface IMsgOfflineMessageList {
	OfflineMessageList: IMsgOfflineMessageListPart[];
}

export interface IMsgOfflineMessageListPart {
	messageId: number;
	clientUid: Uid;
	subject: string;
	subjectRendered: string;
	timestamp: OffsetDateTime;
	isRead: boolean;
}

export interface IMsgPermList {
	PermList: IMsgPermListPart[];
}

export interface IMsgPermListPart {
	groupIdEnd: Permission;
	permissionId: Permission | undefined;
	permissionName: string | undefined;
	permissionDescription: string | undefined;
}

export interface IMsgServerLog {
	ServerLog: IMsgServerLogPart[];
}

export interface IMsgServerLogPart {
	lastOffset: string;
	fileSize: string;
	log: string;
}

export type InMessage =
	IMsgChannelListFinished
	| IMsgChannelPasswordChanged
	| IMsgChannelDescriptionChanged
	| IMsgChannelPermList
	| IMsgClientChatClosed
	| IMsgClientChatComposing
	| IMsgFiletransferStatus
	| IMsgPluginCommand
	| IMsgFileInfo
	| IMsgFileList
	| IMsgFiletransfer
	| IMsgOfflineMessage
	| IMsgOfflineMessageList
	| IMsgPermList
	| IMsgServerLog

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
		codecLatencyFactor?: number;
		isUnencrypted?: boolean;
		deleteDelay?: RustDuration;
		phoneticName?: string;
	};
}

export interface OChangeChannelAddPerm {
	ChannelAddPerm: {
		id: ChannelId;

		value: number;
		permissionId?: Permission;
		permissionName?: string;
	};
}

export interface OChangeChannelClientAddPerm {
	ChannelClientAddPerm: {
		id: ChannelId;

		clientDbId: ClientDbId;
		value: number;
		permissionId?: Permission;
		permissionName?: string;
	};
}

export interface OChangeChannelClientDelPerm {
	ChannelClientDelPerm: {
		id: ChannelId;

		clientDbId: ClientDbId;
		permissionId?: Permission;
		permissionName?: string;
	};
}

export interface OChangeChannelClientPermListRequest {
	ChannelClientPermListRequest: {
		id: ChannelId;

		clientDbId: ClientDbId;
	};
}

export interface OChangeChannelDelPerm {
	ChannelDelPerm: {
		id: ChannelId;

		permissionId?: Permission;
		permissionName?: string;
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

		parent: ChannelId;
		order: ChannelId;
	};
}

export interface OChangeChannelPermListRequest {
	ChannelPermListRequest: {
		id: ChannelId;

	};
}

export interface OChangeChannelCreateDirectory {
	ChannelCreateDirectory: {
		id: ChannelId;

		password: string;
		path: string;
	};
}

export interface OChangeChannelDeleteFile {
	ChannelDeleteFile: {
		id: ChannelId;

		password: string;
		path: string;
	};
}

export interface OChangeChannelFileListRequest {
	ChannelFileListRequest: {
		id: ChannelId;

		password: string;
		path: string;
	};
}

export interface OChangeChannelRenameFile {
	ChannelRenameFile: {
		id: ChannelId;

		password: string;
		fromPath: string;
		toPath: string;
		toChannel?: ChannelId;
		toChannelPassword?: string;
	};
}

export interface OChangeClientAddPerm {
	ClientAddPerm: {
		id: ClientId;

		value: number;
		skip: boolean;
		permissionId?: Permission;
		permissionName?: string;
	};
}

export interface OChangeClientConnectionInfoRequest {
	ClientConnectionInfoRequest: {
		id: ClientId;

	};
}

export interface OChangeClientDelPerm {
	ClientDelPerm: {
		id: ClientId;

		permissionId?: Permission;
		permissionName?: string;
	};
}

export interface OChangeClientPermListRequest {
	ClientPermListRequest: {
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

export interface OChangeConnectionClientUpdate {
	ConnectionClientUpdate: {
		
		name?: string;
		phoneticName?: string;
		inputMuted?: boolean;
		outputMuted?: boolean;
		inputHardwareEnabled?: boolean;
		outputHardwareEnabled?: boolean;
		isChannelCommander?: boolean;
		isRecording?: boolean;
		avatarHash?: string;
		away?: string | null;
		talkPowerRequest?: string | null;
	};
}

export interface OChangeConnectionRemove {
	ConnectionRemove: {
		
	};
}

export interface OChangeConnectionPluginCommandRequest {
	ConnectionPluginCommandRequest: {
		
		name: string;
		data: string;
		target: PluginTargetMode;
		targetClientId: ClientId;
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

		channel: ChannelId;
		password?: string;
	};
}

export interface OChangeClientKick {
	ClientKick: {
		id: ClientId;

		reason: Reason;
		reasonMessage?: string;
	};
}

export interface OChangeServerFileListRequest {
	ServerFileListRequest: {
		
		path: string;
	};
}

export interface OChangeServerDeleteFile {
	ServerDeleteFile: {
		
		path: string;
	};
}

export interface OChangeServerPermListRequest {
	ServerPermListRequest: {
		
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

export interface OChangeServerLogView {
	ServerLogView: {
		
		lines?: number;
		reverse?: boolean;
		instanceLog?: boolean;
		offset?: string;
	};
}

export interface OChangeServerEdit {
	ServerEdit: {
		
		password?: string | null;
		name?: string;
		welcomeMessage?: string;
		maxClients?: number;
		hostmessage?: string;
		hostmessageMode?: HostMessageMode;
		hostbannerUrl?: string;
		hostbannerGfxUrl?: string;
		hostbannerGfxInterval?: RustDuration;
		hostbuttonTooltip?: string;
		hostbuttonUrl?: string;
		hostbuttonGfxUrl?: string;
		icon?: IconId;
		hostbannerMode?: HostBannerMode;
		nickname?: string;
		codecEncryptionMode?: CodecEncryptionMode;
		defaultServerGroup?: ServerGroupId;
		defaultChannelGroup?: ChannelGroupId;
		prioritySpeakerDimmModificator?: number;
		phoneticName?: string;
		tempChannelDefaultDeleteDelay?: RustDuration;
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
	| OChangeChannelAddPerm
	| OChangeChannelClientAddPerm
	| OChangeChannelClientDelPerm
	| OChangeChannelClientPermListRequest
	| OChangeChannelDelPerm
	| OChangeChannelDescriptionRequest
	| OChangeChannelMove
	| OChangeChannelPermListRequest
	| OChangeChannelCreateDirectory
	| OChangeChannelDeleteFile
	| OChangeChannelFileListRequest
	| OChangeChannelRenameFile
	| OChangeClientAddPerm
	| OChangeClientConnectionInfoRequest
	| OChangeClientDelPerm
	| OChangeClientPermListRequest
	| OChangeClientVariablesRequest
	| OChangeClientEdit
	| OChangeConnectionClientUpdate
	| OChangeConnectionRemove
	| OChangeConnectionPluginCommandRequest
	| OChangeClientAddServerGroup
	| OChangeClientRemoveServerGroup
	| OChangeClientMove
	| OChangeClientKick
	| OChangeServerFileListRequest
	| OChangeServerDeleteFile
	| OChangeServerPermListRequest
	| OChangeServerVariablesRequest
	| OChangeServerConnectionInfoRequest
	| OChangeServerLogView
	| OChangeServerEdit
	| OChangeServerGroupAddClient
	| OChangeServerGroupRemoveClient
;
