import type monaco from "monaco-editor";

type ThemeColor = {
    label: string;
    token: string;
    background: boolean;
    group?: Mp.ColorKey;
};

export const ColorTokens: { [key in Mp.ColorKey]: string } = {
    color: "editor.foreground",
    background: "editor.background",
    caret: "editor.cursor.foreground",
    lineHightlightBackground: "editor.lineHighlight.background",
    lineHightlightBorder: "editor.lineHighlight.border",
    lineNumber: "editor.lineNumber.foreground",
    space: "editor.whitespace.foreground",
    bracketMatchBackground: "editor.bracketMatch.background",
    bracketMatchBorder: "editor.bracketMatch.border",
    selection: "editor.selection.foreground",
    selectionBackground: "editor.selection.background",
    search: "editor.findMatch.foreground",
    searchBackground: "editor.findMatch.background",
    searchHighlight: "editor.findMatch.highlight.foreground",
    searchHighlightBackground: "editor.findMatch.highlight.background",
    link: "editorLink.activeForeground",
};

/* -------------------------------- Begin vs theme -------------------------------- */
export const lihgt_colors: Mp.IColors = {
    [ColorTokens["background"]]: "#fffffe",
    [ColorTokens["color"]]: "#000000",
    [ColorTokens["caret"]]: "#000000",
    [ColorTokens["lineHightlightBackground"]]: "#fffffe",
    [ColorTokens["lineHightlightBorder"]]: "#eeeeee",
    [ColorTokens["lineNumber"]]: "#237893",
    [ColorTokens["space"]]: "rgba(51, 51, 51, 0.2)",
    [ColorTokens["bracketMatchBackground"]]: "rgba(0, 100, 0, 0.1)",
    [ColorTokens["bracketMatchBorder"]]: "#b9b9b9",
    [ColorTokens["selection"]]: "#000000",
    [ColorTokens["selectionBackground"]]: "#add6ff",
    [ColorTokens["search"]]: "#000000",
    [ColorTokens["searchBackground"]]: "#a8ac94",
    [ColorTokens["searchHighlight"]]: "#000000",
    [ColorTokens["searchHighlightBackground"]]: "rgba(234, 92, 0, 0.33)",
    [ColorTokens["link"]]: "#0000ff",
};
export const vs: monaco.editor.IStandaloneThemeData = {
    base: "vs",
    inherit: true,
    rules: [{ token: "", foreground: "000000", background: "fffffe" }],
    colors: lihgt_colors,
};
/* -------------------------------- End vs theme -------------------------------- */

/* -------------------------------- Begin vs-dark theme -------------------------------- */
export const dark_colors = {
    [ColorTokens["background"]]: "#1E1E1E",
    [ColorTokens["color"]]: "#D4D4D4",
    [ColorTokens["caret"]]: "#aeafad",
    [ColorTokens["lineHightlightBackground"]]: "#1E1E1E",
    [ColorTokens["lineHightlightBorder"]]: "#282828",
    [ColorTokens["lineNumber"]]: "#858585",
    [ColorTokens["space"]]: "rgba(227, 228, 226, 0.16)",
    [ColorTokens["bracketMatchBackground"]]: "rgba(0, 100, 0, 0.1)",
    [ColorTokens["bracketMatchBorder"]]: "#888888",
    [ColorTokens["selection"]]: "#D4D4D4",
    [ColorTokens["selectionBackground"]]: "#264f78",
    [ColorTokens["search"]]: "#D4D4D4",
    [ColorTokens["searchBackground"]]: "#515c6a",
    [ColorTokens["searchHighlight"]]: "#D4D4D4",
    [ColorTokens["searchHighlightBackground"]]: "rgba(234, 92, 0, 0.33)",
    [ColorTokens["link"]]: "#4e94ce",
};
export const vs_dark: monaco.editor.IStandaloneThemeData = {
    base: "vs-dark",
    inherit: true,
    rules: [{ token: "", foreground: "D4D4D4", background: "1E1E1E" }],
    colors: dark_colors,
};
//F8F8F2
/* -------------------------------- End vs-dark theme -------------------------------- */

export const getThemeData = (theme: Mp.Theme, colors: Mp.IColors): monaco.editor.IStandaloneThemeData => {
    return {
        base: theme == "dark" ? "vs-dark" : "vs",
        inherit: true,
        rules: [{ token: "", foreground: colors["color"], background: colors["background"] }],
        colors,
    };
};

export const Colors: { [key: string]: ThemeColor } = {
    color: {
        label: "Text",
        token: "editor.foreground",
        background: false,
        group: "background",
    },
    caret: {
        label: "Caret",
        background: false,
        token: "editorCursor.foreground",
    },
    lineHightlightBackground: {
        label: "Current Line Background",
        background: true,
        token: "editor.lineHighlightBackground",
    },
    lineHightlightBorder: {
        label: "Current Line",
        background: false,
        token: "editor.lineHighlightBorder",
    },
    lineNumber: {
        label: "Line Number",
        background: false,
        token: "editorLineNumber.foreground",
    },
    space: {
        label: "Space",
        background: false,
        token: "editorWhitespace.foreground",
    },
    bracketMatchBackground: {
        label: "Bracket Background",
        background: true,
        token: "editorBracketMatch.background",
    },
    bracketMatchBorder: {
        label: "Bracket Border",
        background: false,
        token: "editorBracketMatch.border",
    },
    selection: {
        label: "Selection",
        background: false,
        token: "editor.selectionForeground",
        group: "selectionBackground",
    },
    search: {
        label: "Matched",
        background: false,
        token: "editor.findMatchForeground",
        group: "searchBackground",
    },
    searchHighlight: {
        label: "Matched Highlight",
        background: false,
        token: "editor.findMatchHighlightForeground",
        group: "searchHighlightBackground",
    },
    link: {
        label: "Link",
        background: false,
        token: "editorLink.activeForeground",
    },
};
