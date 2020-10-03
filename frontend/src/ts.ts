export type ChannelId = number;
export type ChannelGroupId = number;
export type ClientId = number;
export type ServerGroupId = number;

export enum Codec {
	SpeexNarrowband = "SpeexNarrowband",
	SpeexWideband = "SpeexWideband",
	SpeexUltrawideband = "SpeexUltrawideband",
	CeltMono = "CeltMono",
	OpusVoice = "OpusVoice",
	OpusMusic = "OpusMusic",
}

export function codecToName(codec: Codec) {
	switch (codec) {
		case Codec.SpeexNarrowband: return "Speex Narrowband";
		case Codec.SpeexWideband: return "Speex Wideband";
		case Codec.SpeexUltrawideband: return "Speex Ultrawideband";
		case Codec.CeltMono: return "Celt Mono";
		case Codec.OpusVoice: return "Opus Voice";
		case Codec.OpusMusic: return "Opus Music";
		default: return "Unknown";
	}
}

export enum ChannelType {
	Permanent = "Permanent",
	SemiPermanent = "SemiPermanent",
	Temporary = "Temporary"
}
