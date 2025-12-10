import { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
import { AVAILABLE_ENCODING, OS } from "./constants";
import { IPCBase } from "./ipc";

const ipc = new IPCBase();

class Util {
    isWin() {
        return navigator.userAgent.includes(OS.windows);
    }

    toCommandLineArgs(filePath?: string, grepRequest?: Mp.GrepRequest, position?: Mp.Position): string {
        const args = [filePath];
        if (grepRequest) {
            args.push("-g");
            args.push(grepRequest.condition);
            args.push(grepRequest.start_directory);
            args.push(grepRequest.file_type ? grepRequest.file_type : "*.*");
            if (grepRequest.match_by_word) {
                args.push("-m");
            }
            if (grepRequest.case_sensitive) {
                args.push("-c");
            }
            if (grepRequest.regexp) {
                args.push("-r");
            }
            if (grepRequest.recursive) {
                args.push("-s");
            }
        } else if (position) {
            args.push(position.x.toString());
            args.push(position.y.toString());
        }

        return args.join(" ");
    }

    getFileMenubarItems(settings: Mp.Settings, encoding: string): Mp.MenuItem[] {
        return [
            {
                id: "New",
                label: "New",
                type: "text",
                accel: "Ctrl+N",
            },
            {
                id: "Open",
                label: "Open",
                type: "text",
                accel: "Ctrl+O",
            },
            {
                id: "Save",
                label: "Save",
                type: "text",
                accel: "Ctrl+S",
            },
            {
                id: "SaveAs",
                label: "SaveAs",
                type: "text",
            },
            {
                id: "Print",
                label: "Print",
                type: "text",
                accel: "Ctrl+P",
            },
            {
                type: "separator",
            },
            {
                id: "encoding",
                label: "Encoding",
                type: "submenu",
                submenuId: "encoding",
                items: AVAILABLE_ENCODING.map((enc) => {
                    return {
                        id: "encoding",
                        label: enc,
                        type: "radio",
                        checked: enc == encoding,
                        value: enc,
                        submenuId: "encoding",
                    };
                }),
            },
            {
                type: "separator",
            },
            {
                id: "History",
                label: "History",
                type: "submenu",
                submenuId: "History",
                items: settings.history
                    .map((history) => {
                        return {
                            id: "History",
                            label: history,
                            type: "text",
                            value: history,
                            submenuId: "History",
                        } as Mp.MenuItem;
                    })
                    .concat([
                        {
                            type: "separator",
                        },
                        {
                            id: "clearHistory",
                            label: "Clear Opened",
                            type: "text",
                            submenuId: "History",
                        },
                    ]),
            },
        ];
    }

    getViewMenubarItems(settings: Mp.Settings, textType: Mp.TextType): Mp.MenuItem[] {
        const preference = settings.preference[textType];
        return [
            {
                id: "ShowLineNumber",
                label: "Show Line Number",
                type: "check",
                checked: preference.showLineNumber,
            },
            {
                id: "AutoIndent",
                label: "Auto Indent",
                type: "check",
                checked: preference.autoIndent,
            },
            {
                id: "Wordwrap",
                label: "Wordwrap",
                type: "check",
                checked: preference.wordWrap,
            },
            {
                id: "renderWhitespace",
                type: "submenu",
                label: "Render White Space",
                submenuId: "renderWhitespace",
                items: [
                    {
                        id: "renderWhitespace",
                        type: "radio",
                        label: "None",
                        value: "none",
                        checked: "none" == preference.renderWhitespace,
                        submenuId: "renderWhitespace",
                    },
                    {
                        id: "renderWhitespace",
                        type: "radio",
                        label: "Selection",
                        value: "selection",
                        checked: "selection" == preference.renderWhitespace,
                        submenuId: "renderWhitespace",
                    },
                    {
                        id: "renderWhitespace",
                        type: "radio",
                        label: "All",
                        value: "all",
                        checked: "all" == preference.renderWhitespace,
                        submenuId: "renderWhitespace",
                    },
                ],
            },
            {
                id: "lineHighlight",
                type: "check",
                label: "Line Highlight",
                submenuId: "lineHighlight",
                checked: preference.lineHighlight,
            },
            {
                type: "separator",
            },
            {
                id: "fontSize",
                label: "Font Size",
                type: "submenu",
                submenuId: "fontSize",
                items: [
                    {
                        id: "fontSize",
                        label: "10",
                        type: "radio",
                        value: "10",
                        checked: preference.fontSize == 10,
                        submenuId: "fontSize",
                    },
                    {
                        id: "fontSize",
                        label: "11",
                        type: "radio",
                        value: "11",
                        checked: preference.fontSize == 11,
                        submenuId: "fontSize",
                    },
                    {
                        id: "fontSize",
                        label: "12",
                        type: "radio",
                        value: "12",
                        checked: preference.fontSize == 12,
                        submenuId: "fontSize",
                    },
                    {
                        id: "fontSize",
                        label: "13",
                        type: "radio",
                        value: "13",
                        checked: preference.fontSize == 13,
                        submenuId: "fontSize",
                    },
                    {
                        id: "fontSize",
                        label: "14",
                        type: "radio",
                        value: "14",
                        checked: preference.fontSize == 14,
                        submenuId: "fontSize",
                    },
                ],
            },
            {
                type: "separator",
            },
            {
                id: "indentBySpaces",
                label: "Indent By Space",
                type: "check",
                checked: preference.indentBySpaces,
            },
            {
                id: "indentSize",
                label: "Indent Size",
                type: "submenu",
                submenuId: "indentSize",
                items: [
                    {
                        id: "indentSize",
                        label: "1",
                        type: "radio",
                        value: "1",
                        checked: preference.indentSize == 1,
                        submenuId: "indentSize",
                    },
                    {
                        id: "indentSize",
                        label: "2",
                        type: "radio",
                        value: "2",
                        checked: preference.indentSize == 2,
                        submenuId: "indentSize",
                    },
                    {
                        id: "indentSize",
                        label: "3",
                        type: "radio",
                        value: "3",
                        checked: preference.indentSize == 3,
                        submenuId: "indentSize",
                    },
                    {
                        id: "indentSize",
                        label: "4",
                        type: "radio",
                        value: "4",
                        checked: preference.indentSize == 4,
                        submenuId: "indentSize",
                    },
                ],
            },
            {
                type: "separator",
            },
            {
                id: "Theme",
                label: "Theme",
                type: "submenu",
                submenuId: "Theme",
                items: [
                    {
                        id: "Theme",
                        label: "Light",
                        type: "radio",
                        checked: settings.theme == "light",
                        value: "light",
                        submenuId: "Theme",
                    },
                    {
                        id: "Theme",
                        label: "Dark",
                        type: "radio",
                        checked: settings.theme == "dark",
                        value: "dark",
                        submenuId: "Theme",
                    },
                ],
            },
        ];
    }

    async is_file(target: string): Promise<boolean> {
        return await ipc.invoke("is_file", target);
    }

    async exists(target: string | undefined | null, createIfNotFound = false) {
        if (!target) return false;

        const result = await ipc.invoke("exists", target);

        if (result == false && createIfNotFound) {
            await ipc.invoke("mkdir", target);
        }

        return result;
    }

    toPhysicalPosition = (bounds: Mp.Bounds) => {
        return new PhysicalPosition(bounds.x, bounds.y);
    };

    toPhysicalSize = (bounds: Mp.Bounds) => {
        return new PhysicalSize(bounds.width, bounds.height);
    };

    toBounds = (position: PhysicalPosition, size: PhysicalSize): Mp.Bounds => {
        return {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        };
    };
}

const util = new Util();

export default util;
