import { BASE_ADDRESS, soft_merge } from "./util";

type FilterFlags<Base, Condition> = {
	[Key in keyof Base]: Base[Key] extends Condition ? never : Key
};
type AllowedNames<Base, Condition> = FilterFlags<Base, Condition>[keyof Base];
type SubType<Base, Condition> = Pick<Base, AllowedNames<Base, Condition>>;
type SettGroup = NonNullable<keyof SubType<TransientSettings, Function>>;

export class TransientSettings {
	private _syncDebounceTimer: number | undefined;
	public synth = new TransientSettingsSynth();
	public ui = new TransientSettingsUi();

	public async read_from_proxy() {
		const resp = await fetch(`${BASE_ADDRESS}/transient/*`);
		const data = await resp.json();
		soft_merge(this, data);
	}

	public sync_to_proxy() {
		if (this._syncDebounceTimer === undefined) {
			this._syncDebounceTimer = setTimeout(() => this.sync_to_proxy_async(), 5000);
		}
	}

	public flush() {
		if (this._syncDebounceTimer !== undefined) {
			this.sync_to_proxy_async();
		}
	}

	private async sync_to_proxy_async(group?: SettGroup) {
		this._syncDebounceTimer = undefined;

		let [path, obj] = group !== undefined
			? [group, this[group]]
			: ["*", this];

		await fetch(`${BASE_ADDRESS}/transient/${path}`, {
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

class TransientSettingsUi {
	public showSidebar: boolean = true;
	public showChat: boolean = true;
	public showDescription: boolean = true;
}

export const transientSettings: TransientSettings = new TransientSettings();
