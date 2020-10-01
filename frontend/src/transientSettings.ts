import { soft_merge } from "./util";
import { MessageTarget } from "./ts";
import { Connection } from "./connection";
import { backend } from "./backend/backend";

type FilterFlags<Base, Condition> = {
	[Key in keyof Base]: Base[Key] extends Condition ? never : Key
};
type AllowedNames<Base, Condition> = FilterFlags<Base, Condition>[keyof Base];
type SubType<Base, Condition> = Pick<Base, AllowedNames<Base, Condition>>;
type SettGroup = NonNullable<keyof SubType<TransientSettings, Function>>;

export class TransientSettings {
	private _syncDebounceTimer: number | undefined;
	private _syncDebounceGroup: SettGroup | undefined;
	public synth = new TransientSettingsSynth();
	public ui = new TransientSettingsUi();
	public chat = new TransientSettingsChat(this);

	public async loadAsync() {
		const resp = await backend.fetch(`/transient/*`);
		const data = await resp.json();
		soft_merge(this, data);
	}

	public save(group?: SettGroup) {
		if (this._syncDebounceTimer === undefined) {
			this._syncDebounceGroup = group;
			this._syncDebounceTimer = setTimeout(() => this.saveAsync(), 5000);
		} else if (this._syncDebounceGroup !== group) {
			this._syncDebounceGroup = undefined;
		}
	}

	public flush() {
		if (this._syncDebounceTimer !== undefined) {
			this.saveAsync();
		}
	}

	private async saveAsync() {
		this._syncDebounceTimer = undefined;
		const group = this._syncDebounceGroup;

		let [path, obj] = group !== undefined
			? [group, this[group]]
			: ["*", this];

		await backend.fetch(`/transient/${path}`, {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(obj, (k, v) => k.startsWith('_') ? undefined : v)
		});
	}
}

export class TransientSettingsSynth {
	public voiceId?: string;
	public volume: number = 1;
	public speed: number = 1;
	private _voiceIdCache?: string
	private _voiceCache?: SpeechSynthesisVoice
	public get voice(): SpeechSynthesisVoice | undefined {
		if (this._voiceIdCache !== this.voiceId) {
			this._voiceIdCache = this.voiceId;
			const voices = window.speechSynthesis.getVoices();
			this._voiceCache = voices.find(v => v.voiceURI === this.voiceId) ?? voices.find(_ => true);
		}
		return this._voiceCache;
	}
	public set voice(v: SpeechSynthesisVoice | undefined) {
		if (v) {
			this._voiceCache = v;
			this._voiceIdCache = v.voiceURI;
			this.voiceId = v.voiceURI;
		} else {
			this._voiceIdCache = undefined;
			this._voiceCache = undefined;
			this.voiceId = undefined;
		}
	}

	public getNewUtter(): SpeechSynthesisUtterance {
		const utter = new SpeechSynthesisUtterance();
		if (this.voice) utter.voice = this.voice;
		if (this.speed !== undefined) utter.rate = this.speed;
		if (this.volume !== undefined) utter.volume = this.volume;
		return utter;
	}
}

export class TransientSettingsUi {
	public showSidebar: boolean = true;
	public showChat: boolean = true;
	public showDescription: boolean = true;
}

export class TransientSettingsChat {
	private _parent: TransientSettings;

	constructor(parent: TransientSettings) {
		this._parent = parent;
	}

	public set(text: string | undefined, target: MessageTarget, con: Connection) {
		const key = MessageTarget.toUniqueString(target, con);
		if (key === undefined) return;
		const oldVal = (this as any)[key];
		const storeText = !text ? null : text;
		if (storeText !== oldVal) {
			(this as any)[key] = storeText;
			this._parent.save("chat");
		}
	}

	public get(target: MessageTarget, con: Connection): string | undefined {
		const key = MessageTarget.toUniqueString(target, con);
		if (key === undefined) return undefined;
		return (this as any)[key] ?? undefined;
	}
}
