import util from "./util";
import { IPCBase } from "./ipc";

const ipc = new IPCBase();

class Helper {
    private initialized = false;
    private watchTarget = "";

    onMainReady = async (dropTagetId: string): Promise<Mp.ReadyEvent> => {
        const args = await ipc.invoke("get_args", undefined);
        const locale = args.locales.some((locale) => locale.toLowerCase().includes("ja")) ? "ja" : "en";
        window.lang = locale;

        if (!this.initialized) {
            await ipc.invoke("prepare_menu", undefined);
            await ipc.invoke("listen_file_drop", dropTagetId);
        }

        this.initialized = true;

        if (args.file?.file_path) {
            this.startWatch(args.file.file_path);
        }

        return {
            mode: args.grep ? "grep" : "editor",
            filePath: args.file?.file_path,
            content: args.file?.content,
            grep: args.grep,
            startLine: args.file?.start_line ? { x: args.file?.start_line.column, y: args.file?.start_line.row } : undefined,
            locale,
            encoding: args.file?.encoding,
            restorePosition: args.restore_position,
        };
    };

    showErrorMessage = async (ex: any | string) => {
        if (typeof ex == "string") {
            await ipc.invoke("message", { dialog_type: "message", message: ex, kind: "error" });
        } else {
            await ipc.invoke("message", { dialog_type: "message", message: ex.message, kind: "error" });
        }
    };

    openContextMenu = async (position: Mp.Position) => {
        await ipc.invoke("open_list_context_menu", position);
    };

    confirm = async (message: string, buttons?: string[]): Promise<Mp.MessageResult> => {
        return await ipc.invoke("message", { dialog_type: "ask", message, kind: "warning", ok_label: buttons ? buttons[0] : "Yes", cancel_label: buttons ? buttons[1] : "No" });
    };

    saveFile = async (fullPath: string, data: string, encoding: string) => {
        try {
            await ipc.invoke("write_text_file", { fullPath, data, encoding });
            return true;
        } catch (ex: any) {
            await this.showErrorMessage(ex);
            return false;
        }
    };

    openFile = async () => {
        try {
            return await ipc.invoke("show_open_dialog", { dialog_type: "ask", message: "" });
        } catch (ex: any) {
            this.showErrorMessage(ex);
        }
    };

    readFile = async (filePath: string) => {
        try {
            const readResult = await ipc.invoke("read_text_file", filePath);
            return {
                file_path: filePath,
                content: readResult.content,
                encoding: readResult.encoding,
            };
        } catch (ex: any) {
            this.showErrorMessage(ex);
        }
    };

    openNewWindow = async (filePath: string, grepRequest?: Mp.GrepRequest, position?: Mp.Position) => {
        await ipc.invoke("new_window", util.toCommandLineArgs(filePath, grepRequest, position));
    };

    startWatch = async (target: string) => {
        await this.abortWatch();
        this.watchTarget = target;
        ipc.invoke("watch", this.watchTarget);
    };

    abortWatch = async () => {
        if (this.watchTarget) {
            await ipc.invoke("unwatch", this.watchTarget);
        }
    };

    grep = async (request: Mp.GrepRequest) => {
        const results = await ipc.invoke("run_grep", request);
        return results.sort((a, b) => a.full_path.localeCompare(b.full_path) || a.line_number - b.line_number);
    };

    abortGrep = async () => {
        await ipc.invoke("abort_grep", undefined);
    };

    changeEncoding = async (filePath: string, encoding: string) => {
        return await ipc.invoke("change_encoding", { file_path: filePath, encoding });
    };

    changeTheme = async (theme: Mp.Theme) => {
        await ipc.invoke("change_theme", theme);
    };

    showSaveDialog = async (title: string, defaultPath: string): Promise<string | null> => {
        return await ipc.invoke("show_save_dialog", { title, default_path: defaultPath, dialog_type: "ask", message: "" });
    };

    getUrlsFromClipboard = async (): Promise<Mp.PasteData> => {
        const failedResult = { fullPaths: [] };

        const uriAvailable = await ipc.invoke("is_uris_available", undefined);

        if (!uriAvailable) return failedResult;

        const data = await ipc.invoke("read_uris", undefined);
        if (!data.urls.length) return failedResult;

        return { fullPaths: data.urls };
    };

    getTextFromClipboard = async () => {
        return await ipc.invoke("read_clipboard_text", undefined);
    };

    writeTextToClipboard = async (text: string) => {
        await ipc.invoke("write_clipboard_text", text);
    };

    unlistenAll = async () => {
        await this.abortWatch();
        await ipc.invoke("unlisten_file_drop", undefined);
    };
}

const helper = new Helper();

export default helper;
