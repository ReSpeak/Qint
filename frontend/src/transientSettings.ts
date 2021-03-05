import { get, writable } from "svelte/store";
import { deep_diff, deep_merge } from "./util";
import { backend } from "./backend/backend";
import { NodeSelection } from "./app";

export const enum DescriptionMode {
	None = "None",
	Info = "Info",
	Files = "Files",
}

export class TransientSettings {
	private _syncDebounceTimer: number | undefined;
	/// Value from last save
	private _lastSave: any;
	public synth = new TransientSettingsSynth();
	public ui = new TransientSettingsUi();
	public chat = new TransientSettingsChat(this);
	public app = new TransientSettingsApp();
	public audio = new TransientSettingsAudio();
	public shortcuts = new TransientSettingsShortcuts(this);

	public async loadAsync() {
		try {
			const resp = await backend.fetch(`/transient`);
			const data = await resp.json();
			this._lastSave = data;
			deep_merge(this, data);
		} catch (e) {
			console.error("Failed to load transient settings", e);
		}
	}

	public save() {
		if (this._syncDebounceTimer === undefined)
			this._syncDebounceTimer = setTimeout(() => this.saveAsync(), 5000);
	}

	public flush() {
		if (this._syncDebounceTimer !== undefined)
			this.saveAsync();
	}

	private async saveAsync() {
		this._syncDebounceTimer = undefined;

		let newSave = JSON.parse(JSON.stringify(this, (k, v) => k.startsWith('_') ? undefined : v));
		// Diff to last save
		let diff = deep_diff(this._lastSave, newSave);

		this._lastSave = newSave;

		try {
			await backend.fetch(`/transient`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(diff),
			});
		} catch (e) {
			console.error("Failed to save transient settings", e);
		}
	}
}

const synth: SpeechSynthesis | undefined = window.speechSynthesis;

export class TransientSettingsSynth {
	public voiceId?: string;
	public volume: number = 1;
	public speed: number = 1;
	private _voiceIdCache?: string
	private _voiceCache?: SpeechSynthesisVoice
	private _previousUtter: SpeechSynthesisUtterance | undefined;
	public get voice(): SpeechSynthesisVoice | undefined {
		if (this._voiceIdCache !== this.voiceId) {
			if (synth) {
				const voices = synth.getVoices();
				this._voiceCache = voices.find(v => v.voiceURI === this.voiceId) ?? voices.find(_ => true);
				this._voiceIdCache = this.voiceId;
			} else {
				this.voice = undefined;
			}
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

	public readonly canSpeak = synth !== undefined;

	private getNewUtter(): SpeechSynthesisUtterance {
		const utter = new SpeechSynthesisUtterance();
		if (this.voice) utter.voice = this.voice;
		if (this.speed !== undefined) utter.rate = this.speed;
		if (this.volume !== undefined) utter.volume = this.volume;
		return utter;
	}

	public trySpeak(text: string) {
		if (synth) {
			const utter = this.getNewUtter();
			utter.text = text;
			// Due to a weird bug when calling
			// speak(..), cancel(), speak(..)
			// the second speak will be canceled too.
			// This is a weird workaround for that.
			if (synth.speaking && this._previousUtter) {
				this._previousUtter.onend = () => {
					synth.speak(utter);
				}
				synth.cancel();
			} else {
				synth.speak(utter);
			}
			this._previousUtter = utter;
		}
	}

	public getVoices(): SpeechSynthesisVoice[] {
		if (synth) {
			return synth.getVoices();
		} else {
			return [];
		}
	}
}

// TODO move into own app.ui management
export class TransientSettingsUi {
	private get descriptionMode() { return get(this._descriptionMode); }
	private set descriptionMode(val: DescriptionMode) { this._descriptionMode.set(val); }
	private get developMode() { return get(this._developMode); }
	private set developMode(val: boolean) { this._developMode.set(val); }
	public readonly _descriptionMode = writable(DescriptionMode.None);
	public readonly _developMode = writable(false);
	/// If the default state is muted for new connections
	public defaultInputMuted: boolean = false;
	public defaultOutputMuted: boolean = false;

	toJSON() {
		const res: any = {};
		for (const k in this) {
			res[k] = this[k];
		}
		return res;
	}
}
Object.defineProperty(TransientSettingsUi.prototype, 'descriptionMode', {enumerable: true});
Object.defineProperty(TransientSettingsUi.prototype, 'developMode', {enumerable: true});

export class TransientSettingsChat {
	private _parent: TransientSettings;

	constructor(parent: TransientSettings) {
		this._parent = parent;
	}

	public save(text: string | undefined, selection: NodeSelection) {
		const key = selection.uniqueStr;
		if (key === undefined) return;
		const oldVal = (this as any)[key];
		const storeText = !text ? null : text;
		if (storeText !== oldVal) {
			(this as any)[key] = storeText;
			this._parent.save();
		}
	}

	public load(selection: NodeSelection): string | undefined {
		const key = selection.uniqueStr;
		if (key === undefined) return undefined;
		return (this as any)[key] ?? undefined;
	}
}

export class TransientSettingsApp {
	public askBeforeClosing: boolean = true;
	public allowBrowserNotifications: boolean | undefined = undefined;
}

export class TransientSettingsAudio {
	public globalVolume: number = 1.0;
	public loudnessThreshold: number | null = null;
}

export const enum Tristate {
	True = "True",
	False = "False",
	Toggle = "Toggle",
}

export type ShortcutAction = { Away: Tristate }
			| { InputMute: Tristate }
			| { OutputMute: Tristate };

export interface Shortcut {
	keycode: string;
	action: ShortcutAction;
}

export class TransientSettingsShortcuts {
	private _parent: TransientSettings;
	public actions: Shortcut[] = [];

	constructor(parent: TransientSettings) {
		this._parent = parent;
	}

	public addShortcut(shortcut: Shortcut) {
		if (!shortcut.action || !shortcut.keycode) {
			console.log(`Not saving incomplete shortcut: ${JSON.stringify(shortcut)}`);
			return;
		}
		this.actions.push(shortcut);
		this._parent.save();
	}

	private static actionEquals(a: ShortcutAction, b: ShortcutAction): boolean {
		if ("Away" in a && "Away" in b)
			return a.Away === b.Away;
		if ("InputMute" in a && "InputMute" in b)
			return a.InputMute === b.InputMute;
		if ("OutputMute" in a && "OutputMute" in b)
			return a.OutputMute === b.OutputMute;
		return false;
	}

	public deleteShortcut(shortcut: Shortcut) {
		for (let i = 0; i < this.actions.length; i++) {
			const s = this.actions[i];
			if (s.keycode === shortcut.keycode && TransientSettingsShortcuts.actionEquals(s.action, shortcut.action)) {
				this.actions.splice(i, 1);
				this._parent.save();
				break;
			}
		}
	}
}
