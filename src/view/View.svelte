<script lang="ts">
    import { onMount, tick } from "svelte";
    import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { appState, dispatch, initSettings, settings, temporal, textState, updatePreferences } from "./appStateReducer.svelte";
    import { BROWSER_SHORTCUT_KEYS, DEFAULT_ENCODING, OS, SINGLE_BROWSER_SHORTCUT_KEYS } from "../constants";
    import { IPC } from "../ipc";
    import helper from "../helper";
    import util from "../util";
    import { path } from "../path";

    import Bar from "./Bar.svelte";
    import Editor from "./Editor.svelte";
    import WatchDialog from "./WatchDialog.svelte";
    import GrepDialog from "./GrepDialog.svelte";
    import GrepProgress from "./GrepProgressDialog.svelte";
    import Statusbar from "./Statusbar.svelte";
    import Settings from "../settings";
    import Preference from "./Preference.svelte";

    const ipc = new IPC("View");
    let settingStore = new Settings();

    // Linux only
    let handleKeyUp = false;
    let ready = $state(false);
    let emitted = false;

    const openContextMenu = async (e: MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        closeMenu();
        if (navigator.userAgent.includes(OS.windows)) {
            await helper.openContextMenu({ x: e.screenX, y: e.screenY });
        } else {
            await helper.openContextMenu({ x: e.clientX, y: e.clientY });
        }
    };

    const toggleMaximize = async () => {
        const view = WebviewWindow.getCurrent();
        const maximized = await view.isMaximized();
        if (maximized) {
            view.unmaximize();
            view.setPosition(util.toPhysicalPosition(settings.bounds));
        } else {
            const position = await view.innerPosition();
            const size = await view.innerSize();
            settings.bounds = util.toBounds(position, size);
            await view.maximize();
        }
        settings.isMaximized = !maximized;
        dispatch({ type: "isMaximized", value: !$appState.isMaximized });
    };

    const onWindowSizeChanged = async () => {
        const isMaximized = await WebviewWindow.getCurrent().isMaximized();
        dispatch({ type: "isMaximized", value: isMaximized });
    };

    const setTitle = async () => {
        await WebviewWindow.getCurrent().setTitle($appState.fullPath);
    };

    const handleContextMenuEvent = async (e: Mp.ContextMenuEvent) => {
        switch (e.id) {
            case "New":
                await helper.openNewWindow("");
                break;
            case "Open": {
                await openFile();
                break;
            }
            case "Print":
                console.log("print");
                break;

            case "Theme":
                const theme = e.value == "dark" ? "dark" : "light";
                settings.theme = theme;
                await helper.changeTheme(theme);
                onSettingsChange();
                break;

            case "ShowLineNumber":
                console.log(settings.preference[textState.textType].showLineNumber);
                temporal[textState.textType].showLineNumber = !temporal[textState.textType].showLineNumber;
                console.log(settings.preference[textState.textType].showLineNumber);
                break;
            case "AutoIndent":
                temporal[textState.textType].autoIndent = !temporal[textState.textType].autoIndent;
                break;
            case "Wordwrap":
                temporal[textState.textType].wordWrap = !temporal[textState.textType].wordWrap;
                break;
            case "fontSize":
                temporal[textState.textType].fontSize = Number(e.value);
                break;
            case "lineHighlight":
                temporal[textState.textType].lineHighlight = !temporal[textState.textType].lineHighlight;
                break;

            case "indentBySpaces": {
                temporal[textState.textType].indentBySpaces = !temporal[textState.textType].indentBySpaces;
                break;
            }
            case "indentSize": {
                temporal[textState.textType].indentSize = Number(e.value);
                break;
            }
            case "renderWhitespace": {
                temporal[textState.textType].renderWhitespace = e.value as Mp.WhiteSpaceRenderMode;
                break;
            }

            case "Grep":
                startGrep();
                break;

            case "encoding": {
                changeEncoding(e.value);
                break;
            }

            case "History": {
                if (e.value) {
                    await tryOpenFile(e.value);
                }
                break;
            }
            case "clearHistory":
                clearHistory();
                break;

            case "copyFilePath":
                helper.writeTextToClipboard($appState.fullPath);
                break;

            case "preference":
                showPreference();
                break;
        }
    };

    const onclick = () => {
        if ($appState.openingMenu) {
            dispatch({ type: "openingMenu", value: false });
            return;
        }
        closeMenu();
    };

    const onkeyup = async (e: KeyboardEvent) => {
        if (navigator.userAgent.includes(OS.windows)) return;

        if (!handleKeyUp) return;

        if (e.key == "Control") {
            return;
        }

        handleKeyUp = false;
    };

    const onkeydown = async (e: KeyboardEvent) => {
        if (e.ctrlKey) {
            handleKeyUp = true;
        }

        if (e.key == "Escape") {
            closeMenu();
        }

        if ((e.ctrlKey && BROWSER_SHORTCUT_KEYS.includes(e.key)) || SINGLE_BROWSER_SHORTCUT_KEYS.includes(e.key)) {
            e.preventDefault();
            e.stopPropagation();
        }

        if (e.ctrlKey) {
            switch (e.key) {
                case "n":
                    e.preventDefault();
                    await helper.openNewWindow("");
                    return;
                case "o":
                    e.preventDefault();
                    await openFile();
                    return;
                case "g":
                    e.preventDefault();
                    startGrep();
                    return;
                case "w":
                    e.preventDefault();
                    showPreference();
                    return;
            }
        }
    };

    const closeMenu = () => {
        if ($appState.visibleMenubarItem) {
            dispatch({ type: "hoverMenuItemGroup", value: "" });
            dispatch({ type: "visibleMenubarItem", value: "" });
        }
    };

    const showPreference = () => {
        if ($appState.mode == "grep") return;
        dispatch({ type: "showPreference", value: true });
    };

    const startGrep = () => {
        dispatch({ type: "showGrepDialog", value: true });
    };

    const executeGrep = async (request: Mp.GrepRequest) => {
        dispatch({ type: "grepRequest", value: request });

        if ($appState.fullPath || $appState.isDirty) {
            return await openNewWindow("", request, undefined);
        }

        dispatch({ type: "mode", value: "grep" });
        dispatch({ type: "showGrepProgress", value: true });
        const results = await helper.grep(request);
        dispatch({ type: "grepResult", value: results });
        dispatch({ type: "showGrepProgress", value: false });
        await ipc.sendTo("View", "grep_end", {});
    };

    const abortGrep = async () => {
        await helper.abortGrep();
    };

    const tryOpenFile = async (filePath: string) => {
        const found = await util.exists(filePath);
        if (!found) {
            const result = await helper.confirm("File no longer exists. Do you remove this from history?");
            if (result.button == "Yes") {
                const index = settings.history.findIndex((item) => item == filePath);
                settings.history.splice(index, 1);
            }
            return;
        }
        await openFile(filePath);
    };

    const openFile = async (filePath?: string) => {
        const data = filePath ? await helper.readFile(filePath) : await helper.openFile();
        if (!data) return;

        if (!$appState.fullPath && !$appState.isDirty && $appState.mode != "grep") {
            await loadFileContent(data.file_path, data.content, data.encoding);
        } else {
            await openNewWindow(data.file_path);
        }
    };

    const openNewWindow = async (filePath: string, grepRequest?: Mp.GrepRequest, position?: Mp.Position) => {
        await helper.openNewWindow(filePath, grepRequest, position);
    };

    const loadFileContent = async (filePath: string, content: string, encoding: string) => {
        textState.encoding = encoding;
        dispatch({ type: "init", value: { filePath, content, mode: "editor" } });
        await helper.startWatch(filePath);
        updateHistory(filePath);
        await ipc.sendTo("View", "load", false);
        await setTitle();
    };

    const onFileDrop = async (e: Mp.FileDropEvent) => {
        if (!e.paths.length) return;
        await openFile(e.paths.shift());
        e.paths.forEach((filePath) => helper.openNewWindow(filePath));
    };

    /* Replace reserved/disallowed characters */
    const getNewFileName = () =>
        $appState.content
            .slice(0, 100)
            .split(/\n|\r\n/)
            .find((a) => a)
            ?.replaceAll(/<|>|:|"|\/|\\|\||\?|\*|%/g, "");

    const save = async (saveAs: boolean) => {
        return saveAs ? await trySaveAs() : await trySaveFile();
    };

    const trySaveFile = async () => {
        let target: string | null = $appState.fullPath;
        if (!target) {
            target = await helper.showSaveDialog("", `${getNewFileName()}.txt`);
        }
        if (!target) {
            return false;
        }

        const saved = await helper.saveFile(target, $appState.content, textState.encoding);
        if (saved) {
            dispatch({ type: "isDirty", value: false });
            dispatch({ type: "fullPath", value: target });
            updateHistory(target);
        }
        return saved;
    };

    const trySaveAs = async () => {
        const fileName = $appState.fullPath ? path.basename($appState.fullPath) : `${getNewFileName()}.txt`;
        const target = await helper.showSaveDialog("", fileName);

        if (!target) {
            return false;
        }

        const saved = await helper.saveFile(target, $appState.content, textState.encoding);
        if (saved) {
            dispatch({ type: "isDirty", value: false });
            dispatch({ type: "fullPath", value: target });
            updateHistory(target);
        }
        return saved;
    };

    const changeEncoding = async (encoding?: string) => {
        if (!encoding) return;
        if ($appState.mode == "grep") return;
        if (encoding == textState.encoding) return;

        /* Before save, change encoding value only */
        if (!$appState.fullPath) {
            textState.encoding = encoding;
            return;
        }

        /* Otherwise read file content with the requested encoding */
        const reopen = await helper.confirm("Encoding is being changed. Do you reopen this file?");
        if (reopen.cancelled || reopen.button == "No") return;

        if ($appState.isDirty) {
            const shouldSave = await helper.confirm("Changes will be discarded. Do you save this file before?");
            if (shouldSave.cancelled) return;

            if (shouldSave.button == "Yes") {
                const saved = await trySaveFile();
                if (!saved) return;
            }
        }
        applyEncoding(encoding);
    };

    const applyEncoding = async (encoding: string) => {
        try {
            const content = await helper.changeEncoding($appState.fullPath, encoding);
            textState.encoding = encoding;
            dispatch({ type: "content", value: content });
            ipc.sendTo("View", "encoding_changed", {});
        } catch (ex: any) {
            helper.showErrorMessage(ex);
        }
    };

    const updateHistory = (filePath: string) => {
        if (settings.history.includes(filePath)) {
            return;
        }

        if (settings.history.length == 100) {
            settings.history.splice(0, 1);
        }

        settings.history.push(filePath);
    };

    const clearHistory = () => {
        settings.history = [];
    };

    const unwatch = async () => {
        await helper.abortWatch();
    };

    const beforeClose = async () => {
        if (!$appState.isDirty) return close();

        const shouldSave = await helper.confirm(`${path.basename($appState.fullPath)} is changed. Do you want to save?`);

        if (shouldSave.cancelled) return;

        if (shouldSave.button == "No") return close();

        const saved = await trySaveFile();
        if (saved) return close();
    };

    const close = async () => {
        const view = WebviewWindow.getCurrent();
        if (!settings.isMaximized) {
            const position = await view.innerPosition();
            const size = await view.innerSize();
            settings.bounds = util.toBounds(position, size);
        }
        settings.grepHistory = $appState.grepRequest;
        await helper.unlistenAll();
        settingStore.data = settings;
        await settingStore.save();
        await view.close();
    };

    const onSettingsChange = async () => {
        emitted = true;
        settingStore.update(settings);
        await settingStore.save();
        await settingStore.emit();
    };

    const onWatchEvent = async (e: Mp.WatchEvent) => {
        if (emitted) {
            emitted = false;
            return;
        }

        if (e.file_path == settingStore.watchFile) {
            const theme = settings.theme;
            await settingStore.reload();
            updatePreferences(settingStore.data);
            ipc.sendTo("View", "refelect_settings", {});
            if (theme != settings.theme) {
                await helper.changeTheme(settings.theme);
            }
        }
    };

    const prepare = async () => {
        const e = await helper.onMainReady("root");

        await settingStore.init(e.appDataDir);
        initSettings(settingStore.data);
        textState.encoding = e.encoding ?? DEFAULT_ENCODING;
        await helper.startWatch(settingStore.watchFile);

        if (e.filePath) {
            updateHistory(e.filePath);
        }

        dispatch({ type: "init", value: { filePath: e.filePath ?? "", content: e.content ?? "", mode: e.mode, startLine: e.startLine } });
        await helper.changeTheme(settings.theme);

        await setTitle();
        await tick();

        ready = true;

        if (e.grep) {
            executeGrep(e.grep);
        }

        const webview = WebviewWindow.getCurrent();
        await webview.setSize(util.toPhysicalSize(settings.bounds));
        if (e.restorePosition) {
            await webview.setPosition(util.toPhysicalPosition(settings.bounds));
        }
        await webview.show();
    };

    onMount(() => {
        prepare();
        ipc.receiveTauri("tauri://resize", onWindowSizeChanged);
        ipc.receive("contextmenu_event", handleContextMenuEvent);
        ipc.receiveTauri<Mp.FileDropEvent>("tauri://drag-drop", onFileDrop);
        ipc.receive("settingChanged", onSettingsChange);
        ipc.receive("watch_event", onWatchEvent);

        return () => {
            ipc.release();
        };
    });
</script>

<svelte:window oncontextmenu={openContextMenu} />
<svelte:document {onkeydown} {onkeyup} {onclick} ondragover={(e) => e.preventDefault()} />

<div class="viewport" class:full-screen={$appState.isFullScreen}>
    {#if ready}
        <Bar {beforeClose} {toggleMaximize} />
        {#if $appState.showWatchDialog}
            <WatchDialog />
        {/if}
        {#if $appState.showGrepDialog}
            <GrepDialog {executeGrep} showErrorMessage={(msg) => helper.showErrorMessage(msg)} />
        {/if}
        {#if $appState.showGrepProgress}
            <GrepProgress {abortGrep} />
        {/if}
        {#if $appState.showPreference}
            <Preference />
        {/if}

        <Editor
            startLine={$appState.startLine}
            getClipboardUrls={() => helper.getUrlsFromClipboard()}
            getClipboardText={() => helper.getTextFromClipboard()}
            {save}
            {openNewWindow}
            {unwatch}
            {startGrep}
        />
        <Statusbar />
    {/if}
</div>
