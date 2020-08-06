import { BASE_ADDRESS, soft_merge } from "../util";

export class TransientSettings {
	public synth = new TransientSettingsSynth();

	public async read_from_proxy() {
		const resp = await fetch(`${BASE_ADDRESS}/transient/*`);
		const data = await resp.json();
		soft_merge(this, data);
	}

	public async sync_to_proxy() {
		await fetch(`${BASE_ADDRESS}/transient/*`, {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(this, (k, v) => k.startsWith('_') ? undefined : v)
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
