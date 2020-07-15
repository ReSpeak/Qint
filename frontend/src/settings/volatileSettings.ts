export class VolatileSettings {
	public synth = new VolatileSettingsSynth();
}

export class VolatileSettingsSynth {
	public voiceName?: string;
	public volume: number = 1;
	public speed: number = 1;
	private _voiceCacheName: string | undefined
	private _voiceCache: SpeechSynthesisVoice | undefined
	public get voice(): SpeechSynthesisVoice | null {
		if (this._voiceCacheName !== this.voiceName) {
			this.voiceName = this._voiceCacheName;
			const voices = window.speechSynthesis.getVoices();
			this._voiceCache = voices.find(v => v.name === this.voiceName);
			this._voiceCache = this._voiceCache ?? voices.find(x => true);
		}
		return this._voiceCache ?? null;
	}
	public set voice(v: SpeechSynthesisVoice | null) {
		if (v) {
			this._voiceCache = v;
			this._voiceCacheName = v.name;
			this.voiceName = v.name;
		} else {
			this._voiceCacheName = undefined;
			this._voiceCache = undefined;
			this.voiceName = undefined;
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
