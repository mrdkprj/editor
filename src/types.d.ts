declare global {
    interface Window {
        lang: Mp.LocaleName;
    }

    type RendererName = "View";

    type MainChannelEventMap = {
        minimize: Mp.AnyEvent;
        "toggle-maximize": Mp.AnyEvent;
        close: Mp.AnyEvent;
        "open-list-context-menu": Mp.Position;
    };

    type RendererChannelEventMap = {
        load: Mp.AnyEvent;
        "after-toggle-maximize": Mp.SettingsChangeEvent;
        contextmenu_event: Mp.ContextMenuEvent;
        watch_event: Mp.WatchEvent;
        watch_confirm_event: Mp.WatchConfirmEvent;
        grep_progress: Mp.GrepProgress;
        grep_end: Mp.AnyEvent;
        dialog: boolean;
        encoding_changed: Mp.AnyEvent;
    };

    namespace Mp {
        type Theme = "dark" | "light" | "system";
        type Mode = "editor" | "grep" | "none";
        type TextType = "plain" | "code";
        type WhiteSpaceRenderMode = "none" | "all" | "boundary" | "selection" | "trailing" | undefined;

        type TextState = {
            textType: Mp.TextType;
            encoding: string;
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
            indentSize: number;
            indentBySpaces: null;
            encoding: string;
            clearHistory: null;
            copyFilePath: null;
            fontSize: number;
            renderWhitespace: null;
            lineHighlight: null;
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

        type Settings = {
            bounds: Bounds;
            isMaximized: boolean;
            history: string[];
            theme: Mp.Theme;
            grepHistory: Mp.GrepRequest;
            preference: { [key in TextType]: Preference };
        };

        type Preference = {
            indentSize: number;
            indentBySpaces: boolean;
            showLineNumber: boolean;
            autoIndent: boolean;
            wordWrap: boolean;
            fontSize: number;
            renderWhitespace: WhiteSpaceRenderMode;
            lineHighlight: boolean;
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
