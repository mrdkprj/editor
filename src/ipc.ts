import { listen, emit, UnlistenFn, once, emitTo, EventName } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

type TauriCommand<Req, Res> = {
    Request: Req;
    Response: Res;
};

type WriteFileInfo = {
    fullPath: string;
    data: string;
    encoding?: string;
};

type DialogOptions = {
    dialog_type: "message" | "confirm" | "ask";
    title?: string;
    kind?: "info" | "warning" | "error";
    ok_label?: string;
    cancel_label?: string;
    message: string;
    default_path?: string;
};

type InitArgs = {
    file?: {
        file_path?: string;
        content?: string;
        encoding?: string;
        start_line?: {
            column: number;
            row: number;
        };
    };
    grep?: Mp.GrepRequest;
    locales: string[];
    restore_position: boolean;
    app_data_dir: string;
};

type ReadResult = {
    content: string;
    encoding: string;
};

type TauriCommandMap = {
    prepare_menu: TauriCommand<undefined, undefined>;
    open_list_context_menu: TauriCommand<Mp.Position, undefined>;
    new_window: TauriCommand<string, undefined>;
    exists: TauriCommand<string, boolean>;
    is_uris_available: TauriCommand<undefined, boolean>;
    read_uris: TauriCommand<undefined, Mp.ClipboardData>;
    read_clipboard_text: TauriCommand<undefined, string>;
    write_clipboard_text: TauriCommand<string, undefined>;
    mkdir: TauriCommand<string, undefined>;
    mkdir_all: TauriCommand<string, undefined>;
    create: TauriCommand<string, undefined>;
    read_text_file: TauriCommand<string, ReadResult>;
    write_text_file: TauriCommand<WriteFileInfo, undefined>;
    watch: TauriCommand<string, undefined>;
    unwatch: TauriCommand<string, undefined>;
    message: TauriCommand<DialogOptions, Mp.MessageResult>;
    show_open_dialog: TauriCommand<DialogOptions, Mp.OpenFileResult | null>;
    show_save_dialog: TauriCommand<DialogOptions, string | null>;
    get_args: TauriCommand<undefined, InitArgs>;
    register_drop_target: TauriCommand<undefined, undefined>;
    listen_file_drop: TauriCommand<string, undefined>;
    unlisten_file_drop: TauriCommand<undefined, undefined>;
    change_theme: TauriCommand<Mp.Theme, undefined>;
    run_grep: TauriCommand<Mp.GrepRequest, Mp.GrepResult[]>;
    abort_grep: TauriCommand<undefined, undefined>;
    is_file: TauriCommand<string, boolean>;
    change_encoding: TauriCommand<Mp.EncodeArg, string>;
};

export class IPCBase {
    invoke = async <K extends keyof TauriCommandMap>(channel: K, data: TauriCommandMap[K]["Request"]): Promise<TauriCommandMap[K]["Response"]> => {
        return await invoke<TauriCommandMap[K]["Response"]>(channel, {
            payload: data,
        });
    };
}

export class IPC extends IPCBase {
    private label: string;
    private funcs: UnlistenFn[] = [];

    constructor(label: RendererName) {
        super();
        this.label = label;
    }

    receiveOnce = async <K extends keyof RendererChannelEventMap>(channel: K, handler: (e: RendererChannelEventMap[K]) => void) => {
        const fn = await once<RendererChannelEventMap[K]>(channel, (e) => handler(e.payload), { target: { kind: "WebviewWindow", label: this.label } });
        this.funcs.push(fn);
    };

    receive = async <K extends keyof RendererChannelEventMap>(channel: K, handler: (e: RendererChannelEventMap[K]) => void) => {
        const fn = await listen<RendererChannelEventMap[K]>(channel, (e) => handler(e.payload), { target: { kind: "WebviewWindow", label: this.label } });
        this.funcs.push(fn);
    };

    receiveAny = async <K extends keyof RendererChannelEventMap>(channel: K, handler: (e: RendererChannelEventMap[K]) => void) => {
        const fn = await once<RendererChannelEventMap[K]>(channel, (e) => handler(e.payload), { target: { kind: "Any" } });
        this.funcs.push(fn);
    };

    receiveTauri = async <T>(event: EventName, handler: (e: T) => void) => {
        const fn = await listen<T>(event, (e) => handler(e.payload), {
            target: { kind: "WebviewWindow", label: this.label },
        });
        this.funcs.push(fn);
    };

    send = async <K extends keyof RendererChannelEventMap>(channel: K, data: RendererChannelEventMap[K]) => {
        await emit(channel, data);
    };

    sendTo = async <K extends keyof RendererChannelEventMap>(rendererName: RendererName, channel: K, data: RendererChannelEventMap[K]) => {
        await emitTo({ kind: "WebviewWindow", label: rendererName }, channel, data);
    };

    release = () => {
        this.funcs.forEach((fn) => fn());
    };
}
