import { DEFAULT_ENCODING, DEFAULT_GREP_REQUEST, DEFAULT_PREFERENCE } from "../constants";
import Deferred from "../deferred";
import { defaultSettings } from "../settings";
import { writable } from "svelte/store";

// Linux only
type ContextMenuState = {
    deferred: Deferred<number> | null;
};
const contextMenuState: ContextMenuState = $state({ deferred: null });
export const awaitContextMenu = async () => {
    contextMenuState.deferred = new Deferred();
    await contextMenuState.deferred.promise;
};
export const resolveContextMenu = () => {
    if (contextMenuState.deferred) {
        contextMenuState.deferred.resolve(0);
        contextMenuState.deferred = null;
    }
};

type CusorPosition = {
    line: number;
    column: number;
};

type AppState = {
    content: string;
    isMaximized: boolean;
    isFullScreen: boolean;
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
    showPreference: boolean;
    anyDialogOpened: boolean;
};

export const initialAppState: AppState = {
    content: "",
    isMaximized: false,
    isFullScreen: false,
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
    showPreference: false,
    anyDialogOpened: false,
};

export const textState: Mp.TextState = $state({ textType: "plain", encoding: DEFAULT_ENCODING });
export const contentState: Mp.ContentState = $state({
    mode: "none",
    isDirty: false,
    fullPath: "",
});

/* Settings */
export const settings: Mp.Settings = $state(defaultSettings);
export const initSettings = (data: Mp.Settings) => {
    settings.bounds = data.bounds;
    settings.grepHistory = data.grepHistory;
    settings.history = data.history;
    settings.isMaximized = data.isMaximized;
    settings.preference = data.preference;
    settings.theme = data.theme;
    settings.color = data.color;
    settings.tabMode = data.tabMode;
    temporal.code = data.preference["code"];
    temporal.plain = data.preference["plain"];
};
export const updatePreferences = (data: Mp.Settings) => {
    settings.theme = data.theme;
    settings.color = data.color;
    temporal.code = data.preference["code"];
    temporal.plain = data.preference["plain"];
};
export const temporal: Mp.TypedPreference = $state({
    plain: DEFAULT_PREFERENCE,
    code: DEFAULT_PREFERENCE,
});

/* Preference */
type SelectedPreferenceTab = {
    tab: Mp.PreferenceTab;
};
export const selectedPreference: SelectedPreferenceTab = $state({ tab: "appearance" });

/* Webview Tabs */
type Tabs = {
    webviews: Mp.WebviewTab[];
    scrollLeft: number;
};
export const tabs: Tabs = $state({ webviews: [], scrollLeft: 0 });

/* Tab Drag State */
type TabState = {
    startLabel: string;
    willStartDrag: boolean;
    dragging: boolean;
    lastX: number;
};
export const tabState = $state<TabState>({ startLabel: "", willStartDrag: false, dragging: false, lastX: 0 });

/* Grep Progress */
type GrepProgress = {
    file: string;
    total: number;
    current: number;
};
export const grepProgress = $state<GrepProgress>({ file: "", total: 0, current: 0 });

type AppAction =
    | { type: "mode"; value: Mp.Mode }
    | { type: "init"; value: { filePath: string; content: string; mode: Mp.Mode; startLine?: Mp.Position } }
    | { type: "fullPath"; value: string }
    | { type: "content"; value: string }
    | { type: "isMaximized"; value: boolean }
    | { type: "isDirty"; value: boolean }
    | { type: "openingMenu"; value: boolean }
    | { type: "visibleMenubarItem"; value: string }
    | { type: "watchThisFile"; value: boolean }
    | { type: "suspendWatch"; value: boolean }
    | { type: "grepRequest"; value: Mp.GrepRequest }
    | { type: "grepProgress"; value: Mp.GrepProgress }
    | { type: "grepResult"; value: Mp.GrepResult[] }
    | { type: "columnSelection"; value: boolean }
    | { type: "cusorPosition"; value: CusorPosition }
    | { type: "lineEnding"; value: string }
    | { type: "language"; value: string }
    | { type: "hoverMenuItemGroup"; value: string }
    | { type: "toggleTabMode"; value: boolean }
    | { type: "toggleDialog"; value: { type: Mp.DialogType; open: boolean } }
    | { type: "isFullScreen"; value: boolean };

const updater = (state: AppState, action: AppAction): AppState => {
    switch (action.type) {
        case "mode":
            contentState.mode = action.value;
            return state;

        case "init":
            contentState.fullPath = action.value.filePath;
            contentState.mode = action.value.mode;
            return { ...state, content: action.value.content, startLine: action.value.startLine };

        case "fullPath":
            if (action.value) {
                contentState.fullPath = action.value;
                contentState.mode = "editor";
                return state;
            } else {
                contentState.fullPath = action.value;
                return state;
            }

        case "content":
            return { ...state, content: action.value };

        case "openingMenu":
            return { ...state, openingMenu: action.value };

        case "visibleMenubarItem":
            return { ...state, visibleMenubarItem: action.value };

        case "isDirty":
            contentState.isDirty = action.value;
            return state;

        case "watchThisFile":
            return { ...state, watchThisFile: action.value };

        case "suspendWatch":
            return { ...state, suspendWatch: action.value };

        case "grepRequest":
            return { ...state, grepRequest: action.value };

        case "grepProgress":
            grepProgress.file = action.value.processing;
            grepProgress.current = action.value.current;
            grepProgress.total = action.value.total;
            return state;

        case "grepResult":
            return { ...state, grepResults: action.value };

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

        case "toggleTabMode":
            settings.tabMode = action.value;
            return state;

        case "toggleDialog":
            switch (action.value.type) {
                case "grep":
                    return { ...state, showGrepDialog: action.value.open, anyDialogOpened: action.value.open };
                case "progress":
                    if (action.value.open) {
                        grepProgress.file = "";
                        grepProgress.current = 0;
                        grepProgress.total = 0;
                    }
                    return { ...state, showGrepProgress: action.value.open, anyDialogOpened: action.value.open };
                case "preference":
                    return { ...state, showPreference: action.value.open, anyDialogOpened: action.value.open };
                case "watch":
                    return { ...state, showWatchDialog: action.value.open, anyDialogOpened: action.value.open };
            }

        default:
            return state;
    }
};

const store = writable(initialAppState);

export const dispatch = (action: AppAction) => {
    store.update((state) => updater(state, action));
};

export const appState = store;
