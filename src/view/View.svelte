<script lang="ts">
    import { onMount, tick, untrack } from "svelte";
    import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { appState, dispatch, tabs, contentState, initSettings, settings, temporal, textState, updatePreferences, awaitContextMenu, resolveContextMenu } from "./appStateReducer.svelte";
    import { BROWSER_SHORTCUT_KEYS, DEFAULT_ENCODING, GREP, SINGLE_BROWSER_SHORTCUT_KEYS, UNTITLED } from "../constants";
    import { IPC } from "../ipc";
    import helper from "../helper";
    import util from "../util";
    import path from "../path";

    import Bar from "./Bar.svelte";
    import Editor from "./Editor.svelte";
    import WatchDialog from "./WatchDialog.svelte";
    import GrepDialog from "./GrepDialog.svelte";
    import GrepProgress from "./GrepProgressDialog.svelte";
    import Statusbar from "./Statusbar.svelte";
    import Settings from "../settings";
    import Preference from "./Preference.svelte";
    import TabControl from "./TabControl.svelte";
    import Deferred from "../deferred";
    import GtkResize from "./GtkResize.svelte";
    import { getCurrentWebview } from "@tauri-apps/api/webview";

    const label = getCurrentWebviewWindow().label;
    const ipc = new IPC(label);
    let grepPromise: Deferred<number> | null;
    let settingStore = new Settings();
    let ready = $state(false);
    // Linux only
    let handleKeyUp = false;

    $effect(() => {
        const fullPath = contentState.fullPath;
        const idDirty = contentState.isDirty;
        const mode = contentState.mode;
        untrack(() => updateThisTitle(fullPath, idDirty, mode));
    });

    const openContextMenu = async (e: MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        closeMenu();
        if (util.isWin()) {
            await helper.openContextMenu(label, { x: e.screenX, y: e.screenY });
        } else {
            await awaitContextMenu();
            await helper.openContextMenu(label, { x: e.clientX, y: e.clientY });
        }
    };

    const onmouseup = () => {
        resolveContextMenu();
    };

    const toggleMaximize = async () => {
        if (settings.tabMode) {
            return ipc.invoke("tab_request", { name: "toggleMaximize" });
        }

        const view = getCurrentWebviewWindow();
        const maximized = await view.isMaximized();
        if (maximized) {
            view.unmaximize();
            view.setPosition(util.toPhysicalPosition(settings.bounds));
            view.setSize(util.toPhysicalSize(settings.bounds));
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
        if (settings.tabMode) return;

        const isMaximized = await getCurrentWebviewWindow().isMaximized();
        dispatch({ type: "isMaximized", value: isMaximized });
    };

    const minimize = async () => {
        if (settings.tabMode) {
            return ipc.invoke("tab_request", { name: "minimize" });
        }

        const view = getCurrentWebviewWindow();
        const position = await view.innerPosition();
        const size = await view.innerSize();
        settings.bounds = util.toBounds(position, size);
        await view.minimize();
    };

    const onTabWindowSizeChangeEvent = async (isMaximized: boolean) => {
        dispatch({ type: "isMaximized", value: isMaximized });
    };

    const handleContextMenuEvent = async (e: Mp.ContextMenuEvent) => {
        switch (e.id) {
            case "New": {
                await openNewWindow("");
                break;
            }
            case "Open": {
                await openFile();
                break;
            }
            case "Print":
                window.print();
                break;

            case "Theme":
                const theme = e.value == "dark" ? "dark" : "light";
                settings.theme = theme;
                await helper.changeTheme(theme);
                onSettingsChange();
                break;

            case "ShowLineNumber":
                temporal[textState.textType].showLineNumber = !temporal[textState.textType].showLineNumber;
                break;
            case "indentBySpaces": {
                temporal[textState.textType].indentBySpaces = !temporal[textState.textType].indentBySpaces;
                break;
            }

            case "Wordwrap":
                temporal[textState.textType].wordWrap = !temporal[textState.textType].wordWrap;
                break;
            case "lineHighlight":
                temporal[textState.textType].lineHighlight = !temporal[textState.textType].lineHighlight;
                break;

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
                helper.writeTextToClipboard(contentState.fullPath);
                break;

            case "preference":
                showPreference();
                break;

            case "tab":
                toggleTabMode();
                break;
        }
    };

    const onclick = (e: MouseEvent) => {
        e.preventDefault();
        if ($appState.openingMenu) {
            dispatch({ type: "openingMenu", value: false });
            return;
        }
        closeMenu();
    };

    const onkeyup = async (e: KeyboardEvent) => {
        if (util.isWin()) return;

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
                case "f":
                    e.preventDefault();
                    return;
                case "n":
                    e.preventDefault();
                    await openNewWindow("");
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
                    if (settings.tabMode) {
                        tryClose();
                    } else {
                        showPreference();
                    }
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
        if (contentState.mode == "grep") return;
        dispatch({ type: "toggleDialog", value: { type: "preference", open: true } });
    };

    const startGrep = () => {
        dispatch({ type: "toggleDialog", value: { type: "grep", open: true } });
    };

    const executeGrep = async (request: Mp.GrepRequest) => {
        if (grepPromise) {
            await grepPromise.promise;
        }

        dispatch({ type: "grepRequest", value: request });
        settings.grepHistory = request;

        if (contentState.fullPath || contentState.isDirty) {
            return await openNewWindow("", request, undefined);
        }

        dispatch({ type: "mode", value: "grep" });
        dispatch({ type: "toggleDialog", value: { type: "progress", open: true } });
        const results = await helper.grep(request);
        dispatch({ type: "grepResult", value: results });
        dispatch({ type: "toggleDialog", value: { type: "progress", open: false } });
        await ipc.sendTo(label, "grep_end", {});
        onSettingsChange();
    };

    const onGrepProgress = (e: Mp.GrepProgress) => {
        dispatch({ type: "grepProgress", value: e });
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
        if (!filePath) {
            return await helper.openFile();
        }

        if (!contentState.fullPath && !contentState.isDirty && contentState.mode != "grep") {
            const alreadyOpened = await ipc.invoke("is_file_opened", filePath);
            if (alreadyOpened) {
                return;
            }

            const data = await helper.readFile(filePath);
            if (data) {
                await loadFileContent(data.file_path, data.content, data.encoding);
            }
        } else {
            await openNewWindow(filePath);
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
        await ipc.sendTo(label, "load", false);
    };

    const onFileDrop = async (e: Mp.FileDropEvent) => {
        console.log(document.elementFromPoint(e.position.x, e.position.y));
        console.log(e.paths);
        if (!e.paths.length) return;
        await openFile(e.paths.shift());
        e.paths.forEach((filePath) => openNewWindow(filePath));
    };

    /* Replace reserved/disallowed characters */
    const getNewFileName = () =>
        $appState.content
            ? $appState.content
                  .slice(0, 100)
                  .split(/\n|\r\n/)
                  .find((a) => a)
                  ?.replaceAll(/<|>|:|"|\/|\\|\||\?|\*|%/g, "")
            : UNTITLED;

    const save = async (saveAs: boolean) => {
        return saveAs ? await trySaveAs() : await trySaveFile();
    };

    const trySaveFile = async () => {
        let target: string | null = contentState.fullPath;
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
        const fileName = contentState.fullPath ? path.basename(contentState.fullPath) : `${getNewFileName()}.txt`;
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
        if (contentState.mode == "grep") return;
        if (encoding == textState.encoding) return;

        /* Before save, change encoding value only */
        if (!contentState.fullPath) {
            textState.encoding = encoding;
            return;
        }

        /* Otherwise read file content with the requested encoding */
        const reopen = await helper.confirm("Encoding is being changed. Do you reopen this file?");
        if (reopen.cancelled || reopen.button == "No") return;

        if (contentState.isDirty) {
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
            const content = await helper.changeEncoding(contentState.fullPath, encoding);
            textState.encoding = encoding;
            dispatch({ type: "content", value: content });
            ipc.sendTo(label, "encoding_changed", {});
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
        onSettingsChange();
    };

    const clearHistory = () => {
        settings.history = [];
        onSettingsChange();
    };

    const unwatch = async () => {
        await helper.abortWatch();
    };

    const beforeClose = async () => {
        if (!contentState.isDirty) return destroy();

        const thisWindow = getCurrentWebviewWindow();
        const isMinimized = await thisWindow.isMinimized();
        if (isMinimized) {
            await thisWindow.unminimize();
        }
        const name = contentState.fullPath ? path.basename(contentState.fullPath) : UNTITLED;
        const shouldSave = await helper.confirm(`"${name}" is changed. Do you want to save?`);

        if (shouldSave.cancelled) return cancelCloseAll();
        if (shouldSave.button == "No") return destroy();

        const saved = await trySaveFile();
        if (saved) {
            await destroy();
        } else {
            await cancelCloseAll();
        }
    };

    const tryClose = async () => {
        if (settings.tabMode) {
            await ipc.invoke("tab_request", { name: "closeAll" });
        } else {
            await getCurrentWebviewWindow().close();
        }
    };

    const cancelCloseAll = async () => {
        if (settings.tabMode) {
            await ipc.invoke("tab_request", { name: "cancel" });
        }
    };

    const destroy = async () => {
        const thisWindow = getCurrentWebviewWindow();

        /* On Linux, must move webview back to its original parent window */
        if (settings.tabMode) {
            await ipc.invoke("tab_request", { name: "detach" });
        }

        settingStore.data = $state.snapshot(settings);

        const isMinimized = await thisWindow.isMinimized();
        if (!settings.isMaximized && !isMinimized) {
            const position = await thisWindow.innerPosition();
            const size = await thisWindow.innerSize();
            settingStore.data.bounds = util.toBounds(position, size);
        }

        settingStore.data.grepHistory = $appState.grepRequest;
        await helper.unlistenAll();
        await settingStore.save();

        thisWindow.destroy();
    };

    const onSettingsChange = async () => {
        settingStore.data = $state.snapshot(settings);
        await settingStore.save();
        await ipc.sendOthers("reloadSettings", {});
    };

    const onReloadSettings = async () => {
        const theme = settings.theme;
        await settingStore.reload();
        updatePreferences(settingStore.data);
        await ipc.sendTo(label, "refelect_settings", {});
        if (theme != settings.theme) {
            await helper.changeTheme(settings.theme);
        }
    };

    const saveTabMode = async () => {
        settingStore.data = $state.snapshot(settings);
        settingStore.data.tabMode = !settingStore.data.tabMode;
        await settingStore.save();
        dispatch({ type: "toggleTabMode", value: settingStore.data.tabMode });
    };

    const toggleTabMode = async () => {
        await saveTabMode();
        await ipc.invoke("tab_request", { name: "toggleTabMode", data: settings.tabMode });
    };

    const onTabEvent = async (e: Tab.TabEvent) => {
        switch (e.name) {
            case "activated": {
                getCurrentWebview().setFocus();
                if (grepPromise) {
                    grepPromise.resolve(0);
                    grepPromise = null;
                }
                break;
            }
            case "maximized": {
                onTabWindowSizeChangeEvent(true);
                return;
            }
            case "unmaximized": {
                onTabWindowSizeChangeEvent(false);
                return;
            }
            case "titleChanged": {
                updateTabTitle(e.data);
                break;
            }
            case "reordered": {
                tabs.webviews = e.data;
                break;
            }
            case "closed": {
                let index = tabs.webviews.findIndex((tab) => tab.label == e.data);
                tabs.webviews.splice(index, 1);
                break;
            }
            case "modeChanged": {
                dispatch({ type: "toggleTabMode", value: e.data.tab_mode });
                if (e.data.tabs.length) {
                    tabs.webviews = e.data.tabs;
                }
                break;
            }
            case "added": {
                tabs.webviews.push(e.data);
                break;
            }
            case "close": {
                beforeClose();
                break;
            }
            case "scrolled": {
                tabs.scrollLeft = e.data;
                break;
            }
        }
    };

    const scrollTab = (scrollLeft: number) => {
        tabs.scrollLeft = scrollLeft;
    };

    const updateTabTitle = (e: Mp.WebviewTitle) => {
        tabs.webviews
            .filter((tab) => tab.label == e.label)
            .forEach((tab) => {
                tab.title = e.title;
                tab.path = e.path;
            });
    };

    const updateThisTitle = async (fullPath: string, isDirty: boolean, mode: Mp.Mode) => {
        const title = getTitle(fullPath, isDirty, mode);
        const path = contentState.fullPath;
        const webviewTitle = { label, title, path };
        updateTabTitle(webviewTitle);
        ipc.invoke("update_title", webviewTitle);
        await getCurrentWebviewWindow().setTitle(title);
    };

    const getTitle = (fullPath: string, isDirty: boolean, mode: Mp.Mode) => {
        const mark = isDirty ? "*" : "";
        const title = fullPath ? `${path.basename(fullPath)}${mark}` : mode == "grep" ? `${GREP}${mark}` : `${UNTITLED}${mark}`;
        return title;
    };

    const prepare = async () => {
        const e = await helper.onMainReady("root");

        await settingStore.init(e.appDataDir);
        initSettings(settingStore.data);
        textState.encoding = e.encoding ?? DEFAULT_ENCODING;

        if (e.filePath) {
            updateHistory(e.filePath);
        }

        dispatch({ type: "init", value: { filePath: e.filePath ?? "", content: e.content ?? "", mode: e.mode, startLine: e.startLine } });
        await helper.changeTheme(settings.theme);

        await tick();

        ready = true;

        if (e.grep) {
            if (settings.tabMode) {
                grepPromise = new Deferred(0);
            }
            executeGrep(e.grep);
        }

        const thisWindow = getCurrentWebviewWindow();

        await thisWindow.setSize(util.toPhysicalSize(settings.bounds));
        if (e.restorePosition) {
            await thisWindow.setPosition(util.toPhysicalPosition(settings.bounds));
        }

        if (settings.tabMode) {
            const toggled = await ipc.invoke("tab_request", { name: "toggleTabMode", data: settings.tabMode });
            if (!toggled) {
                await ipc.invoke("tab_request", { name: "add" });
            }
        } else {
            await thisWindow.show();
            await thisWindow.setFocus();
        }
    };

    onMount(() => {
        prepare();
        ipc.receiveTauri("tauri://close-requested", beforeClose);
        ipc.receiveTauri("tauri://resize", onWindowSizeChanged);
        ipc.receive("contextmenu_event", handleContextMenuEvent);
        ipc.receiveTauri<Mp.FileDropEvent>("tauri://drag-drop", onFileDrop);
        ipc.receive("grep_progress", onGrepProgress);
        ipc.receive("tabWindowSizeChange", onTabWindowSizeChangeEvent);
        ipc.receive("settingChanged", onSettingsChange);
        ipc.receive("reloadSettings", onReloadSettings);
        ipc.receive("tab_event", onTabEvent);
        ipc.receive("scrollTab", scrollTab);

        return () => {
            ipc.release();
        };
    });
</script>

<svelte:window oncontextmenu={openContextMenu} />
<svelte:document {onkeydown} {onkeyup} {onclick} {onmouseup} ondragover={(e) => e.preventDefault()} />

<div class="viewport" class:full-screen={$appState.isFullScreen}>
    {#if util.isLinux()}
        <GtkResize />
    {/if}
    {#if ready}
        <Bar {label} {openNewWindow} close={tryClose} {toggleMaximize} {minimize} />
        {#if $appState.showWatchDialog}
            <WatchDialog {label} />
        {/if}
        {#if $appState.showGrepDialog}
            <GrepDialog {label} {executeGrep} showErrorMessage={(msg) => helper.showErrorMessage(msg)} />
        {/if}
        {#if $appState.showGrepProgress}
            <GrepProgress {label} {abortGrep} />
        {/if}
        {#if $appState.showPreference}
            <Preference {label} />
        {/if}
        {#if settings.tabMode}
            <TabControl {label} />
        {/if}
        <div class="editor">
            <Editor
                {label}
                startLine={$appState.startLine}
                getClipboardUrls={() => helper.getUrlsFromClipboard()}
                getClipboardText={() => helper.getTextFromClipboard()}
                {save}
                {openNewWindow}
                {unwatch}
                {startGrep}
            />
        </div>
        <Statusbar />
    {/if}
</div>

<style>
    .editor {
        flex: 1 1 auto;
        overflow: auto;
    }
</style>
