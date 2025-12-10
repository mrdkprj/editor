import { appDataDir, join } from "@tauri-apps/api/path";
import { IPC } from "./ipc";
import { DEFAULT_GREP_REQUEST } from "./constants";

const ipc = new IPC("View");
const SETTING_FILE_NAME = "editor.settings.json";
const EXCEPTION_KEYS = ["history"];

export const defaultSettings: Mp.Settings = {
    bounds: { width: 1200, height: 800, x: 0, y: 0 },
    isMaximized: false,
    history: [],
    theme: "dark",
    grepHistory: DEFAULT_GREP_REQUEST,
    preference: {
        plain: {
            indentSize: 4,
            indentBySpaces: true,
            showLineNumber: true,
            autoIndent: true,
            wordWrap: false,
            fontSize: 14,
            renderWhitespace: "all",
            lineHighlight: true,
        },
        code: {
            indentSize: 4,
            indentBySpaces: true,
            showLineNumber: true,
            autoIndent: true,
            wordWrap: false,
            fontSize: 14,
            renderWhitespace: "selection",
            lineHighlight: true,
        },
    },
};

export default class Settings {
    data = defaultSettings;

    private file = "";
    private dataDir = "";

    async init() {
        this.dataDir = await appDataDir();
        const settingPath = await join(this.dataDir, "temp");
        this.file = await join(settingPath, SETTING_FILE_NAME);
        const fileExists = await ipc.invoke("exists", this.file);

        if (fileExists) {
            try {
                const readResult = await ipc.invoke("read_text_file", this.file);
                if (readResult.content) {
                    this.data = this.createSettings(JSON.parse(readResult.content));
                }
            } catch (ex: any) {
                console.log(ex);
            }
        } else {
            await ipc.invoke("mkdir_all", settingPath);
            await ipc.invoke("create", this.file);
            await ipc.invoke("write_text_file", { fullPath: this.file, data: JSON.stringify(this.data, null, 2) });
        }

        return this.data;
    }

    private createSettings(rawSettings: any): Mp.Settings {
        const config = { ...defaultSettings } as any;

        Object.keys(rawSettings).forEach((key) => {
            if (!(key in config)) return;

            const value = rawSettings[key];

            if (typeof value === "object" && !Array.isArray(value)) {
                Object.keys(value).forEach((valueKey) => {
                    if (valueKey in config[key] || EXCEPTION_KEYS.includes(key)) {
                        config[key][valueKey] = value[valueKey];
                    }
                });
            } else {
                config[key] = value;
            }
        });

        return config;
    }

    getFilePath() {
        return this.file;
    }

    async save() {
        await ipc.invoke("write_text_file", { fullPath: this.file, data: JSON.stringify(this.data, null, 2) });
    }
}
