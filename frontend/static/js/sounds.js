// @ts-check

const ROOT_PATH = "/sound/";

/**
 * @param {string} file
 */
function create(file) {
	if (file.startsWith("http://") || file.startsWith("https://"))
		return new Audio(file);
	else
		return new Audio(ROOT_PATH + file);
}

class QintAudio {
	constructor() {
		this.synth = window.speechSynthesis;

		this.config = {
			/** Text To Speech Enabled */
			tts: false,
			tts_fallback: true
		};

		/** @type {{[name: string]: HTMLAudioElement}} */
		this.sounds = {};
	}

	/**
	 * @param {string} name
	 * @param {string} tts_text
	 */
	async play(name, tts_text) {
		let audio = this.sounds[name];
		if (!audio) {
			audio = create(name);
			this.sounds[name] = audio;
		}
		let sound_ok = true;

		if (!this.config.tts && audio) {
			try {
				await audio.play();
			} catch { sound_ok = false; }
		}
		if (this.config.tts || (!sound_ok && this.config.tts_fallback)) {
			if (!tts_text) return;
			const utter = new SpeechSynthesisUtterance(tts_text);
			// const voices = synth.getVoices();
			//utter.voice = voices[0]; // TODO
			this.synth.cancel();
			this.synth.speak(utter);
		}
	}
}

export default new QintAudio();
