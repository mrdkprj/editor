declare global {
    interface Window {
        lang: Mp.LocaleName;
    }

    type RendererName = "View";

    type RendererChannelEventMap = {
        load: boolean;
        minimize: Mp.AnyEvent;
        toggleMaximize: Mp.AnyEvent;
        tabWindowSizeChange: boolean;
        contextmenu_event: Mp.ContextMenuEvent;
        watch_event: Mp.WatchEvent;
        watch_confirm_event: Mp.WatchConfirmEvent;
        grep_progress: Mp.GrepProgress;
        grep_end: Mp.AnyEvent;
        dialog: boolean;
        encoding_changed: Mp.AnyEvent;
        refelect_settings: Mp.AnyEvent;
        settingChanged: Mp.SettingChangeType;
        reloadSettings: Mp.AnyEvent;
        closeTab: Mp.AnyEvent;
        scrollTab: number;
        dragEnd: Mp.AnyEvent;
        tab_event: Mp.TabEvent;
    };

    namespace Tab {
        type TabEvent =
            | { name: "maximized"; data?: never }
            | { name: "unmaximized"; data?: never }
            | { name: "titleChanged"; data: WebviewTitle }
            | { name: "reordered"; data: Mp.WebviewTitle[] }
            | { name: "closed"; data: string }
            | { name: "modeChanged"; data: { tab_mode: boolean; tabs: WebviewTitle[] } }
            | { name: "close"; data?: never }
            | { name: "scrolled"; data: number }
            | { name: "activated"; data?: never }
            | { name: "added"; data: WebviewTitle };

        type TabRequest =
            | { name: "select"; data: string }
            | { name: "reorder"; data: WebviewTitle[] }
            | { name: "closeAll"; data?: never }
            | { name: "cancel"; data?: never }
            | { name: "update"; data: WebviewTitle }
            | { name: "add"; data?: never }
            | { name: "detach"; data?: never }
            | { name: "minimize"; data?: never }
            | { name: "toggleMaximize"; data?: never }
            | { name: "startDrag"; data?: never }
            | { name: "startResizeDrag"; data: ResizeDirection }
            | { name: "toggleTabMode"; data: boolean };
    }

    namespace Mp {
        type Theme = "dark" | "light" | "system";
        type Mode = "editor" | "grep" | "none";
        type TextType = "plain" | "code";
        type WhiteSpaceRenderMode = "none" | "all" | "boundary" | "selection" | "trailing" | undefined;
        type PreferenceTab = "appearance" | "color";
        type DialogType = "grep" | "progress" | "preference" | "watch";
        type ColorKey =
            | "color"
            | "background"
            | "caret"
            | "lineHightlightBackground"
            | "lineHightlightBorder"
            | "lineNumber"
            | "space"
            | "bracketMatchBackground"
            | "bracketMatchBorder"
            | "selection"
            | "selectionBackground"
            | "search"
            | "searchBackground"
            | "searchHighlight"
            | "searchHighlightBackground"
            | "link";

        type ResizeDirection = "East" | "North" | "NorthEast" | "NorthWest" | "South" | "SouthEast" | "SouthWest" | "West";

        type WebviewTitle = {
            label: string;
            title: string;
            path: string;
        };

        type OpenedWebview = {
            webviews: { [key: string]: WebviewTitle };
        };

        type TextState = {
            textType: Mp.TextType;
            encoding: string;
        };

        type ContentState = {
            mode: Mp.Mode;
            isDirty: boolean;
            fullPath: string;
        };

        type OpenContextMenuRequest = {
            label: string;
            position: Mp.Position;
        };

        type ContextMenuEvent = {
            id: keyof MainContextMenuSubTypeMap;
            value?: string;
        };

        type MainContextMenuSubTypeMap = {
            New: null;
            Open: null;
            Save: null;
            SaveAs: null;
            Print: null;
            History: null;
            Undo: null;
            Redo: null;
            Cut: null;
            Copy: null;
            Paste: null;
            Search: null;
            Replace: null;
            Grep: null;
            ToggleLineComment: null;
            ToggleBlockComment: null;
            AutoIndent: null;
            ShowLineNumber: null;
            Wordwrap: null;
            transformToUppercase: null;
            transformToLowercase: null;
            transformToSnakecase: null;
            transformToCamelcase: null;
            transformToPascalcase: null;
            transformToTitlecase: null;
            transformToKebabcase: null;
            Format: null;
            Theme: null;
            indentBySpaces: null;
            encoding: string;
            clearHistory: null;
            copyFilePath: null;
            renderWhitespace: null;
            lineHighlight: null;
            preference: null;
            tab: null;
        };

        type Bounds = {
            width: number;
            height: number;
            x: number;
            y: number;
        };

        type Position = {
            x: number;
            y: number;
        };

        type Rect = {
            top: number;
            left: number;
            right: number;
            bottom: number;
        };

        type WebviewTab = {
            label: string;
            title: string;
            path: string;
            bounds: Mp.Bounds;
            isMaximized: boolean;
        };

        type UpdateTabsEvent = {
            webviewTitle?: Mp.WebviewTitle;
            tabs?: Mp.WebviewTab[];
        };

        type TypedPreference = { [key in TextType]: Preference };
        type Settings = {
            bounds: Bounds;
            isMaximized: boolean;
            history: string[];
            theme: Mp.Theme;
            grepHistory: Mp.GrepRequest;
            preference: TypedPreference;
            color: { [key in Mp.Theme]: IColors };
            tabMode: boolean;
        };

        type Preference = {
            indentSize: number;
            indentBySpaces: boolean;
            showLineNumber: boolean;
            autoIndent: boolean;
            wordWrap: boolean;
            fontFamily: string;
            fontSize: number;
            renderWhitespace: WhiteSpaceRenderMode;
            lineHighlight: boolean;
        };

        type SettingChangeType = "both" | "preference" | "color";

        type IColors = {
            [colorId in string]: string;
        };

        type OpenFileResult = {
            file_path: string;
            content: string;
            encoding: string;
        };

        type PasteData = {
            fullPaths: string[];
        };

        type ReadyEvent = {
            mode: Mp.Mode;
            filePath?: string;
            content?: string;
            startLine?: Mp.Position;
            grep?: Mp.GrepRequest;
            locale: Mp.LocaleName;
            encoding?: string;
            restorePosition: boolean;
            appDataDir: string;
        };

        type ClipboardData = {
            urls: string[];
        };

        type FileDropEvent = {
            paths: string[];
        };

        type MessageResult = {
            button: string;
            cancelled: boolean;
        };

        type WatchConfirmEvent = {
            applyChange: boolean;
            doNotNotify: boolean;
        };

        type WatchEvent = {
            file_path: string;
            content: string;
            encoding: string;
        };

        type GrepRequest = {
            condition: string;
            start_directory: string;
            file_type: string;
            match_by_word: boolean;
            case_sensitive: boolean;
            regexp: boolean;
            recursive: boolean;
        };

        type GrepResult = {
            full_path: string;
            line_number: number;
            line: string;
            ranges: [number, number][];
        };

        type GrepProgress = {
            processing: string;
            current: number;
            total: number;
        };

        type MenuItem = {
            id?: keyof Mp.MainContextMenuSubTypeMap;
            label?: string;
            type: "text" | "check" | "submenu" | "separator" | "radio";
            items?: Mp.MenuItem[];
            checked?: boolean;
            accel?: string;
            value?: string;
            submenuId?: keyof Mp.MainContextMenuSubTypeMap;
        };

        type EncodeArg = {
            content?: string;
            file_path?: string;
            encoding: string;
        };

        type AnyEvent = {
            args?: any;
        };
    }
}

export {};
