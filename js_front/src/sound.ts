// @ts-check

const ROOT_PATH = "/sound/";

function create(file: string) {
	if (file.startsWith("http://") || file.startsWith("https://"))
		return new Audio(file);
	else
		return new Audio(ROOT_PATH + file);
}

class QintAudio {
	private synth: SpeechSynthesis;
	private config: {
		/** Text To Speech enabled */
		tts: boolean,
		tts_fallback: boolean,
	};
	private sounds: { [name: string]: HTMLAudioElement };

	constructor() {
		console.log("soundo");
		this.synth = window.speechSynthesis;
		this.config = {
			tts: false,
			tts_fallback: true
		};
		this.sounds = {};
	}

	public async play(name: string, ttsText: string) {
		let audio = this.sounds[name];
		if (!audio) {
			audio = create(name);
			this.sounds[name] = audio;
		}
		let soundOk = true;

		if (!this.config.tts && audio) {
			try {
				await audio.play();
			} catch { soundOk = false; }
		}
		if (this.config.tts || (!soundOk && this.config.tts_fallback)) {
			if (!ttsText) return;
			const utter = new SpeechSynthesisUtterance(ttsText);
			// const voices = synth.getVoices();
			// utter.voice = voices[0]; // TODO
			this.synth.cancel();
			this.synth.speak(utter);
		}
	}
}

export default new QintAudio();
