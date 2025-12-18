import { path } from "./path";
import { IPCBase } from "./ipc";
import { DEFAULT_FONT, DEFAULT_GREP_REQUEST } from "./constants";
import { dark_colors, lihgt_colors } from "./theme";

const ipc = new IPCBase();
const SETTING_FILE_NAME = "editor.settings.json";
const WATCH_FILE_NAME = ".watch";

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
            fontFamily: DEFAULT_FONT,
            fontSize: 14,
            renderWhitespace: "all",
            lineHighlight: true,
        },
        code: {
            indentSize: 4,
            indentBySpaces: true,
            showLineNumber: true,
            autoIndent: true,
            fontFamily: DEFAULT_FONT,
            wordWrap: false,
            fontSize: 14,
            renderWhitespace: "selection",
            lineHighlight: true,
        },
    },
    color: {
        light: lihgt_colors,
        dark: dark_colors,
        system: dark_colors,
    },
};

export default class Settings {
    data = defaultSettings;
    watchFile = "";

    private file = "";
    private dataDir = "";
    private settingPath = "";

    async init(appDataDir: string) {
        this.dataDir = appDataDir;
        this.settingPath = path.join(this.dataDir, "temp");
        this.file = path.join(this.settingPath, SETTING_FILE_NAME);
        this.watchFile = path.join(this.settingPath, WATCH_FILE_NAME);
        await this.createFile();

        return this.data;
    }

    private async createFile() {
        const settingsfileExists = await ipc.invoke("exists", this.file);

        if (settingsfileExists) {
            try {
                const readResult = await ipc.invoke("read_text_file", this.file);
                if (readResult.content) {
                    const config = { ...defaultSettings } as any;
                    this.data = this.createSettings(JSON.parse(readResult.content), config);
                }
            } catch (ex: any) {
                console.log(ex);
            }
        } else {
            await ipc.invoke("mkdir_all", this.settingPath);
            await ipc.invoke("create", this.file);
            await ipc.invoke("write_text_file", { fullPath: this.file, data: JSON.stringify(this.data, null, 2) });
        }

        const fileExists = await ipc.invoke("exists", this.watchFile);
        if (!fileExists) {
            await ipc.invoke("create", this.watchFile);
        }
    }

    private createSettings(rawSettings: any, config: any): Mp.Settings {
        Object.keys(rawSettings).forEach((key) => {
            if (!(key in config)) return;

            const value = rawSettings[key];

            if (typeof value === "object" && !Array.isArray(value)) {
                this.createSettings(rawSettings, value);
            } else {
                config[key] = value;
            }
        });

        return config;
    }

    update(data: Mp.Settings) {
        this.data = data;
    }

    async reload() {
        const readResult = await ipc.invoke("read_text_file", this.file);
        if (readResult.content) {
            this.data = JSON.parse(readResult.content);
        }
    }

    async save() {
        await ipc.invoke("write_text_file", { fullPath: this.file, data: JSON.stringify(this.data) });
    }

    async emit() {
        await ipc.invoke("write_text_file", { fullPath: this.watchFile, data: new Date().getTime().toString() });
    }
}
