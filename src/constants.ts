export const handleKeyEvent = () => {
    /**/
};

export const OS = {
    windows: "Windows",
    linux: "Linux",
};

export const SEPARATOR = navigator.userAgent.includes(OS.windows) ? "\\" : "/";
export const DEFAULT_FONT = "Consolas";
export const BROWSER_SHORTCUT_KEYS = ["g", "r", "+", "-", "u", "j"];
export const SINGLE_BROWSER_SHORTCUT_KEYS = ["F7", "F12"];
export const DEFAULT_ENCODING = "UTF-8";
export const AVAILABLE_ENCODING = ["UTF-8", "UTF-16LE", "UTF-16BE", "Shift_JIS", "EUC-JP"];
export const LINE_ENDINGS = {
    0: "LF",
    1: "CRLF",
};
export const DEFAULT_LINE_ENDING = "CRLF";
export const DEFAULT_GREP_REQUEST: Mp.GrepRequest = {
    condition: "",
    start_directory: "",
    file_type: "*.*",
    match_by_word: false,
    case_sensitive: false,
    regexp: false,
    recursive: true,
};
export const DEFAULT_PREFERENCE: Mp.Preference = {
    indentSize: 4,
    indentBySpaces: true,
    showLineNumber: true,
    autoIndent: true,
    wordWrap: false,
    fontFamily: DEFAULT_FONT,
    fontSize: 14,
    renderWhitespace: "selection",
    lineHighlight: true,
};

export const EDIT_MENU_ITEMS: Mp.MenuItem[] = [
    {
        id: "Undo",
        label: "Undo",
        type: "text",
        accel: "Ctrl+Z",
    },
    {
        id: "Redo",
        label: "Redo",
        type: "text",
        accel: "Ctrl+Y",
    },
    {
        type: "separator",
    },
    {
        id: "Cut",
        label: "Cut",
        type: "text",
        accel: "Ctrl+X",
    },
    {
        id: "Copy",
        label: "Copy",
        type: "text",
        accel: "Ctrl+C",
    },
    {
        id: "Paste",
        label: "Paste",
        type: "text",
        accel: "Ctrl+P",
    },
    {
        type: "separator",
    },
    {
        id: "Search",
        label: "Search",
        type: "text",
        accel: "Ctrl+F",
    },
    {
        id: "Replace",
        label: "Replace",
        type: "text",
        accel: "Ctrl+R",
    },
    {
        id: "Grep",
        label: "Grep",
        type: "text",
        accel: "Ctrl+G",
    },
    {
        type: "separator",
    },
    {
        id: "ToggleLineComment",
        label: "Toggle Line Comment",
        type: "text",
        accel: "Ctrl+/",
    },
    {
        id: "ToggleBlockComment",
        label: "Toggle Block Comment",
        type: "text",
    },
];

export const LANGUAGES: { [key: string]: string } = {
    cameligo: "CameLIGO",
    cpp: "C++",
    csharp: "C#",
    csp: "CSP",
    css: "CSS",
    ecl: "ECL",
    fsharp: "F#",
    graphql: "GraphQL",
    hcl: "HCL",
    html: "HTML",
    ini: "ini",
    javascript: "JavaScript",
    mdx: "MDX",
    mysql: "MySQL",
    "objective-c": "Objective-C",
    pgsql: "PostgreSQL",
    php: "PHP",
    qsharp: "Q#",
    scss: "SCSS",
    sparql: "SPARQL",
    sql: "SQL",
    typescript: "TypeScript",
    typespec: "TypeSpec",
    vb: "Visual Basic",
    wgsl: "WGSL",
    xml: "XML",
    yaml: "YAML",
    plaintext: "Plain Text",
};
