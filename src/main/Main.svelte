<script lang="ts">
    import { onMount } from "svelte";
    import { getCurrentWebviewWindow, WebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { Webview } from "@tauri-apps/api/webview";
    import { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
    import { IPC } from "../ipc";
    import util from "../util";
    import { OS } from "../constants";

    const ipc = new IPC("Main");

    const OFF_SCREEN = -30000;
    const isLinux = navigator.userAgent.includes(OS.linux);

    let settings = $state<Mp.Settings>();
    let temporal = $state<Mp.TypedPreference>();

    let currentWebview: Mp.TabbedWebview = { label: "", bounds: { x: 0, y: 0, width: 0, height: 0 }, isMaximized: false };
    let active = false;
    let webviews: Mp.TabbedWebview[] = [];
    let closingLabelSequence: string[] = [];
    let allTabClosing = false;

    const toggleMaximize = async () => {
        const view = getCurrentWebviewWindow();
        const isMaximized = await view.isMaximized();
        if (isMaximized) {
            view.unmaximize();
            view.setPosition(util.toPhysicalPosition(currentWebview.bounds));
        } else {
            const position = await view.innerPosition();
            const size = await view.innerSize();
            currentWebview.bounds = util.toBounds(position, size);
            await view.maximize();
        }

        currentWebview.isMaximized = !isMaximized;

        await ipc.sendOthers("tabWindowSizeChange", !isMaximized);
    };

    const minimize = async () => {
        const view = getCurrentWebviewWindow();
        const position = await view.innerPosition();
        const size = await view.innerSize();
        currentWebview.bounds = util.toBounds(position, size);
        await view.minimize();
    };

    const beforeClose = async () => {
        const isMinimized = await getCurrentWebviewWindow().isMinimized();
        if (isMinimized) {
            await getCurrentWebviewWindow().unminimize();
        }
        await closeAll();
    };

    const closeTab = async (label: string) => {
        allTabClosing = false;
        closingLabelSequence = [label];
        await tryCloseNext();
    };

    const closeAll = async () => {
        if (!active) return;

        allTabClosing = true;
        closingLabelSequence = webviews.map((webview) => webview.label);
        await tryCloseNext();
    };

    const tryCloseNext = async () => {
        const next = closingLabelSequence.pop();
        if (next) {
            await switchTab(next);
            const item = await WebviewWindow.getByLabel(next);
            item?.close();
        }
    };

    const onAnotherWindowClosed = async (closedLabel: string) => {
        if (!allTabClosing) {
            const removed = webviews.findIndex((view) => view.label == closedLabel);
            if (removed > 0) {
                await switchTab(webviews[removed - 1].label);
                webviews.splice(removed, 1);
            }
        } else {
            // If no window to close, hide this before destroy
            if (!closingLabelSequence.length) {
                await hideThis();
            }
            await tryCloseNext();
        }
        ipc.sendTo(closedLabel, "destory", currentWebview);
    };

    const updateTitle = async (e: Mp.UpdateTitleRequest) => {
        if (e.label == currentWebview.label) {
            await getCurrentWebviewWindow().setTitle(e.webviewTitle.title);
        }
    };

    type ResizeEvent = {
        width: number;
        height: number;
    };
    const onResize = async (e: ResizeEvent) => {
        if (active) {
            const toActive = await WebviewWindow.getByLabel(currentWebview.label);
            await toActive?.setSize(new PhysicalSize(e.width, e.height));
        }
    };

    const pushWebiew = async (label: string) => {
        const webviewWindow = await WebviewWindow.getByLabel(label);
        if (!webviewWindow) return;

        const size = await webviewWindow.innerSize();
        const position = await webviewWindow.innerPosition();
        const isMaximized = await webviewWindow.isMaximized();

        webviews.push({ label, bounds: util.toBounds(position, size), isMaximized });

        if (isLinux) {
            // Hide window
            await webviewWindow.hide();
        } else {
            // First place window off-screen
            await webviewWindow.setPosition(new PhysicalPosition(OFF_SCREEN, OFF_SCREEN));
            // Then show window with animation if not visible
            const visible = await webviewWindow.isVisible();
            if (!visible) {
                await webviewWindow.show();
            }
        }

        ipc.sendTo(label, "tabWindowSizeChange", currentWebview.isMaximized);
    };

    const addTab = async (label: string) => {
        if (!active) {
            await startTabMode(label);
            return;
        }

        await pushWebiew(label);
        await ipc.invoke("to_child_window", [label]);
        const labelTitleMap = await ipc.invoke("get_webview_labels", undefined);
        await ipc.sendOthers("enterTabMode", labelTitleMap);
        await switchTab(label);
    };

    const onActivated = (e: Mp.TabActivated) => {};

    const switchTab = async (activeTabLabel: string) => {
        if (activeTabLabel == currentWebview.label) return;

        ipc.sendTo(activeTabLabel, "activateTab", {});

        if (isLinux) {
            const toActive = await Webview.getByLabel(activeTabLabel);
            await toActive?.show();
        } else {
            const size = await getCurrentWebviewWindow().size();
            const toActive = await WebviewWindow.getByLabel(activeTabLabel);
            await toActive?.setPosition(new PhysicalPosition(-8, 0));
            await toActive?.setSize(new PhysicalSize(size.width, size.height));
        }

        if (currentWebview.label) {
            if (isLinux) {
                const toInactive = await Webview.getByLabel(currentWebview.label);
                await toInactive?.hide();
            } else {
                const toInactive = await WebviewWindow.getByLabel(currentWebview.label);
                toInactive?.setPosition(new PhysicalPosition(OFF_SCREEN, OFF_SCREEN));
            }
        }

        currentWebview.label = activeTabLabel;
    };

    const startTabMode = async (initial: string) => {
        webviews = [];
        currentWebview.label = "";

        // Get bounds
        const initiatedWindow = await WebviewWindow.getByLabel(initial);
        if (!initiatedWindow) return;
        const isMaximized = await initiatedWindow.isMaximized();
        const position = await initiatedWindow.innerPosition();
        const size = await initiatedWindow.innerSize();
        currentWebview.bounds = util.toBounds(position, size);
        currentWebview.isMaximized = isMaximized;

        // Set bound
        const thisWindow = getCurrentWebviewWindow();
        await thisWindow.setSize(util.toPhysicalSize(currentWebview.bounds));
        await thisWindow.setPosition(util.toPhysicalPosition(currentWebview.bounds));
        // await getCurrentWebview().setAutoResize(false);
        // Prepare tabs
        const labelTitleMap = await ipc.invoke("get_webview_labels", undefined);
        const sortedLabels = Object.keys(labelTitleMap).toSorted();
        for (const label of sortedLabels) {
            await pushWebiew(label);
        }

        await ipc.invoke("to_child_window", Object.keys(labelTitleMap));
        await ipc.sendOthers("enterTabMode", labelTitleMap);

        await switchTab(initial);

        active = true;

        await thisWindow.show();
        if (currentWebview.isMaximized) {
            await thisWindow.maximize();
        }
    };

    const endTabMode = async () => {
        const thisWindow = getCurrentWebviewWindow();
        // Don't know why but outerPosition is correct
        const position = await thisWindow.outerPosition();
        const size = await thisWindow.innerSize();

        for (const webview of webviews) {
            const bounds = webview.label == currentWebview.label ? util.toBounds(position, size) : webview.bounds;
            const isMaximized = webview.label == currentWebview.label ? currentWebview.isMaximized : webview.isMaximized;

            await ipc.invoke("restore_webview", webview.label);

            const webiewWindow = await WebviewWindow.getByLabel(webview.label);
            if (isLinux) {
                await webiewWindow?.show();
            } else {
                await webiewWindow?.setPosition(util.toPhysicalPosition(bounds));
                await webiewWindow?.setSize(util.toPhysicalSize(bounds));
            }
            await ipc.sendTo(webview.label, "exitTabMode", isMaximized);
        }

        await hideThis();

        // Clean up tabs
        active = false;
        currentWebview.label = "";
        webviews = [];
    };

    const hideThis = async () => {
        const thisWindow = getCurrentWebviewWindow();
        await thisWindow.setPosition(new PhysicalPosition(OFF_SCREEN, OFF_SCREEN));
        await thisWindow.unmaximize();
        await thisWindow.hide();
    };

    onMount(() => {
        ipc.receiveTauri("tauri://close-requested", beforeClose);
        ipc.receiveTauri("tauri://resize", onResize);
        ipc.receive("toggleMaximize", toggleMaximize);
        ipc.receive("minimize", minimize);
        ipc.receive("startTabMode", startTabMode);
        ipc.receive("endTabMode", endTabMode);
        ipc.receive("switchTab", switchTab);
        ipc.receive("updateTitle", updateTitle);
        ipc.receive("addTab", addTab);
        ipc.receive("tabActivated", onActivated);
        ipc.receive("closeTab", closeTab);
        ipc.receive("closeAll", closeAll);
        ipc.receive("closed", onAnotherWindowClosed);

        return () => {
            ipc.release();
        };
    });
</script>

<svelte:document ondragover={(e) => e.preventDefault()} />

<div class="viewport"><div class="title-bar no-print" data-tauri-drag-region={navigator.userAgent.includes(OS.linux) ? true : null}></div></div>

<style>
    .viewport {
        background-color: var(--main-bgcolor);
        color: var(--menu-color);
        height: 100%;
        width: 100%;
        cursor: default;
        display: flex;
        flex-direction: column;
    }

    .title-bar {
        display: flex;
        justify-content: flex-end;
        align-items: center;
        height: 35px;
        min-height: 35px;
        width: 100%;
        color: var(--main-color);
        background-color: var(--bar-bg-color);
        -webkit-app-region: drag;
        user-select: none;
        -webkit-user-select: none;
    }
</style>
