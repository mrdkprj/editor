import { DEFAULT_ENCODING, DEFAULT_GREP_REQUEST } from "../constants";
import { defaultSettings } from "../settings";
import { writable } from "svelte/store";

type CusorPosition = {
    line: number;
    column: number;
};

type AppState = {
    mode: Mp.Mode;
    fullPath: string;
    content: string;
    isDirty: boolean;
    isMaximized: boolean;
    isFullScreen: boolean;
    theme: Mp.Theme;
    openingMenu: boolean;
    visibleMenubarItem: string;
    showWatchDialog: boolean;
    watchThisFile: boolean;
    suspendWatch: boolean;
    showGrepDialog: boolean;
    grepRequest: Mp.GrepRequest;
    grepResults: Mp.GrepResult[];
    showGrepProgress: boolean;
    startLine: Mp.Position | undefined;
    columnSelection: boolean;
    cusorPosition: CusorPosition;
    lineEnding: string;
    language: string;
    hoverMenuItemGroup: string;
};

export const initialAppState: AppState = {
    mode: "none",
    fullPath: "",
    content: "",
    isMaximized: false,
    isFullScreen: false,
    theme: "system",
    isDirty: false,
    openingMenu: false,
    visibleMenubarItem: "",
    showWatchDialog: false,
    watchThisFile: true,
    suspendWatch: false,
    showGrepDialog: false,
    grepRequest: DEFAULT_GREP_REQUEST,
    grepResults: [],
    startLine: undefined,
    showGrepProgress: false,
    columnSelection: false,
    cusorPosition: { line: 0, column: 0 },
    lineEnding: "CRLF",
    language: "",
    hoverMenuItemGroup: "",
};

export const textState: Mp.TextState = $state({ textType: "plain", encoding: DEFAULT_ENCODING });
export const settings: Mp.Settings = $state(defaultSettings);
export const initSettings = (data: Mp.Settings) => {
    settings.bounds = data.bounds;
    settings.grepHistory = data.grepHistory;
    settings.history = data.history;
    settings.isMaximized = data.isMaximized;
    settings.preference = data.preference;
    settings.theme = data.theme;
};

type AppAction =
    | { type: "mode"; value: Mp.Mode }
    | { type: "init"; value: { filePath: string; content: string; mode: Mp.Mode; startLine?: Mp.Position } }
    | { type: "theme"; value: Mp.Theme }
    | { type: "fullPath"; value: string }
    | { type: "content"; value: string }
    | { type: "isMaximized"; value: boolean }
    | { type: "isDirty"; value: boolean }
    | { type: "openingMenu"; value: boolean }
    | { type: "visibleMenubarItem"; value: string }
    | { type: "showWatchDialog"; value: boolean }
    | { type: "watchThisFile"; value: boolean }
    | { type: "suspendWatch"; value: boolean }
    | { type: "showGrepDialog"; value: boolean }
    | { type: "grepRequest"; value: Mp.GrepRequest }
    | { type: "grepResult"; value: Mp.GrepResult[] }
    | { type: "showGrepProgress"; value: boolean }
    | { type: "columnSelection"; value: boolean }
    | { type: "cusorPosition"; value: CusorPosition }
    | { type: "lineEnding"; value: string }
    | { type: "language"; value: string }
    | { type: "hoverMenuItemGroup"; value: string }
    | { type: "isFullScreen"; value: boolean };
const updater = (state: AppState, action: AppAction): AppState => {
    switch (action.type) {
        case "mode":
            return { ...state, mode: action.value };

        case "init":
            return { ...state, fullPath: action.value.filePath, content: action.value.content, mode: action.value.mode, startLine: action.value.startLine };

        case "fullPath":
            if (action.value) {
                return { ...state, fullPath: action.value, mode: "editor" };
            } else {
                return { ...state, fullPath: action.value };
            }

        case "content":
            return { ...state, content: action.value };

        case "theme":
            return { ...state, theme: action.value };

        case "openingMenu":
            return { ...state, openingMenu: action.value };

        case "visibleMenubarItem":
            return { ...state, visibleMenubarItem: action.value };

        case "isDirty":
            return { ...state, isDirty: action.value };

        case "showWatchDialog":
            return { ...state, showWatchDialog: action.value };

        case "watchThisFile":
            return { ...state, watchThisFile: action.value };

        case "suspendWatch":
            return { ...state, suspendWatch: action.value };

        case "showGrepDialog":
            return { ...state, showGrepDialog: action.value };

        case "grepRequest":
            return { ...state, grepRequest: action.value };

        case "grepResult":
            return { ...state, grepResults: action.value };

        case "showGrepProgress":
            return { ...state, showGrepProgress: action.value };

        case "columnSelection":
            return { ...state, columnSelection: action.value };

        case "cusorPosition":
            return { ...state, cusorPosition: action.value };

        case "lineEnding":
            return { ...state, lineEnding: action.value };

        case "language":
            return { ...state, language: action.value };

        case "hoverMenuItemGroup":
            return { ...state, hoverMenuItemGroup: action.value };

        case "isMaximized":
            return { ...state, isMaximized: action.value };

        case "isFullScreen":
            return { ...state, isFullScreen: action.value };

        default:
            return state;
    }
};

const store = writable(initialAppState);

export const dispatch = (action: AppAction) => {
    store.update((state) => updater(state, action));
};

export const appState = store;
