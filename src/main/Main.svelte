<script lang="ts">
    import { onMount } from "svelte";
    import { getCurrentWebviewWindow, WebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { Webview } from "@tauri-apps/api/webview";
    import { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
    import { IPC } from "../ipc";
    import util from "../util";
    import { BROWSER_SHORTCUT_KEYS, OS, SINGLE_BROWSER_SHORTCUT_KEYS } from "../constants";

    const ipc = new IPC("Main");

    const OFF_SCREEN = -30000;
    const isLinux = navigator.userAgent.includes(OS.linux);

    let currentWebviewTab: Mp.WebviewTab = { label: "", title: "", path: "", bounds: { x: 0, y: 0, width: 0, height: 0 }, isMaximized: false };
    let active = false;
    let webviewTabs: Mp.WebviewTab[] = [];
    let closingLabelSequence: string[] = [];
    let allTabClosing = false;

    const toggleMaximize = async () => {
        const window = getCurrentWebviewWindow();
        const isMaximized = await window.isMaximized();
        if (isMaximized) {
            window.unmaximize();
            if (!isLinux) {
                window.setPosition(util.toPhysicalPosition(currentWebviewTab.bounds));
            }
        } else {
            const position = await window.innerPosition();
            const size = await window.innerSize();
            currentWebviewTab.bounds = util.toBounds(position, size);
            await window.maximize();
        }

        currentWebviewTab.isMaximized = !isMaximized;

        await ipc.sendOthers("tabWindowSizeChange", !isMaximized);
    };

    const minimize = async () => {
        const window = getCurrentWebviewWindow();
        const position = await window.innerPosition();
        const size = await window.innerSize();
        currentWebviewTab.bounds = util.toBounds(position, size);
        await window.minimize();
    };

    const onWindowSizeChanged = async () => {
        const isMaximized = await getCurrentWebviewWindow().isMaximized();
        currentWebviewTab.isMaximized = isMaximized;
        await ipc.sendOthers("tabWindowSizeChange", isMaximized);
    };

    type ResizeEvent = {
        width: number;
        height: number;
    };
    const onResize = async (e: ResizeEvent) => {
        if (active) {
            const toActive = await WebviewWindow.getByLabel(currentWebviewTab.label);
            await toActive?.setSize(new PhysicalSize(e.width, e.height));
        }
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
        closingLabelSequence = webviewTabs.map((tab) => tab.label);
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
        const removed = webviewTabs.findIndex((tab) => tab.label == closedLabel);
        webviewTabs.splice(removed, 1);

        if (!allTabClosing) {
            if (webviewTabs.length) {
                if (removed == 0) {
                    await switchTab(webviewTabs[0].label);
                } else {
                    await switchTab(webviewTabs[removed - 1].label);
                }
            }
        } else {
            /* If no window to close, hide this before destroy */
            if (!closingLabelSequence.length) {
                await hideThis();
            }
            await tryCloseNext();
        }
        ipc.sendTo(closedLabel, "destory", currentWebviewTab);
    };

    const updateTabTitle = (e: Mp.WebviewTitle) => {
        webviewTabs
            .filter((tab) => tab.label == e.label)
            .forEach((tab) => {
                tab.title = e.title;
                tab.path = e.path;
            });
    };

    const updateTab = async (e: Mp.UpdateTabsEvent) => {
        if (e.webviewTitle) {
            updateTabTitle(e.webviewTitle);
            if (e.webviewTitle.label == currentWebviewTab.label) {
                await getCurrentWebviewWindow().setTitle(e.webviewTitle.title);
            }
        }

        if (e.tabs) {
            webviewTabs = e.tabs;
        }
    };

    const pushWebiew = async (title: Mp.WebviewTitle) => {
        const webviewWindow = await WebviewWindow.getByLabel(title.label);
        if (!webviewWindow) return;

        const size = await webviewWindow.innerSize();
        const position = await webviewWindow.innerPosition();
        const isMaximized = await webviewWindow.isMaximized();

        webviewTabs.push({ label: title.label, title: title.title, path: title.path, bounds: util.toBounds(position, size), isMaximized });

        if (isLinux) {
            /* Hide window */
            await webviewWindow.hide();
        } else {
            /* First place window off-screen */
            await webviewWindow.setPosition(new PhysicalPosition(OFF_SCREEN, OFF_SCREEN));
            /* Then show window with animation if not visible */
            const visible = await webviewWindow.isVisible();
            if (!visible) {
                await webviewWindow.show();
            }
        }

        ipc.sendTo(title.label, "tabWindowSizeChange", currentWebviewTab.isMaximized);
    };

    const addTab = async (label: string) => {
        if (!active) {
            await startTabMode(label);
            return;
        }

        const openedWebviews = await ipc.invoke("get_webview_labels", undefined);
        await pushWebiew(openedWebviews.webviews[label]);
        /* Making child window comes after WebviewWindow is shown. Otherwise, child window can't get focus */
        await ipc.invoke("to_child_window", [label]);
        await ipc.sendOthers("updateTab", { tabs: webviewTabs });
        /* Delay switching for smooth rendering */
        setTimeout(async () => {
            await switchTab(label, false);
        }, 50);
    };

    const switchTab = async (activeTabLabel: string, hideCurrent = true) => {
        if (activeTabLabel == currentWebviewTab.label) return;

        if (isLinux) {
            const toActive = await Webview.getByLabel(activeTabLabel);
            await toActive?.show();
        } else {
            const size = await getCurrentWebviewWindow().size();
            const toActive = await WebviewWindow.getByLabel(activeTabLabel);
            await toActive?.setPosition(new PhysicalPosition(-8, 0));
            await toActive?.setSize(new PhysicalSize(size.width, size.height));
        }

        if (currentWebviewTab.label) {
            if (isLinux) {
                if (hideCurrent) {
                    const toInactive = await Webview.getByLabel(currentWebviewTab.label);
                    await toInactive?.hide();
                }
            } else {
                const toInactive = await WebviewWindow.getByLabel(currentWebviewTab.label);
                toInactive?.setPosition(new PhysicalPosition(OFF_SCREEN, OFF_SCREEN));
            }
        }

        currentWebviewTab.label = activeTabLabel;

        await activateWebview(activeTabLabel);
    };

    const activateWebview = async (label: string) => {
        /* Make the view restore focus on its editor */
        await ipc.sendTo(label, "tabActivated", {});

        if (isLinux) return;

        /*
            On Windows, focus must be set on active child window.
            But focusing on a child window causes problems and app switching like ALT+Tab does not work.
            So place focus on the active child window's webview instead.
         */
        const activatedWebvivew = await Webview.getByLabel(label);
        await activatedWebvivew?.setFocus();
    };

    const onFocus = () => {
        if (currentWebviewTab.label) {
            activateWebview(currentWebviewTab.label);
        }
    };

    const startTabMode = async (initial: string) => {
        webviewTabs = [];
        currentWebviewTab.label = "";

        const initiatedWindow = await WebviewWindow.getByLabel(initial);
        if (!initiatedWindow) return;
        const isMaximized = await initiatedWindow.isMaximized();
        const position = await initiatedWindow.innerPosition();
        const size = await initiatedWindow.innerSize();
        currentWebviewTab.bounds = util.toBounds(position, size);
        currentWebviewTab.isMaximized = isMaximized;

        const thisWindow = getCurrentWebviewWindow();
        await thisWindow.setSize(util.toPhysicalSize(currentWebviewTab.bounds));
        await thisWindow.setPosition(util.toPhysicalPosition(currentWebviewTab.bounds));
        const openedWebviews = await ipc.invoke("get_webview_labels", undefined);
        const sortedLabels = Object.keys(openedWebviews.webviews).toSorted();
        for (const label of sortedLabels) {
            await pushWebiew(openedWebviews.webviews[label]);
        }

        await ipc.invoke("to_child_window", sortedLabels);
        await ipc.sendOthers("enterTabMode", webviewTabs);

        await switchTab(initial);

        active = true;

        await thisWindow.show();
        if (currentWebviewTab.isMaximized) {
            await thisWindow.maximize();
        }
    };

    const endTabMode = async () => {
        const thisWindow = getCurrentWebviewWindow();
        /* Don't know why but outerPosition is correct */
        const position = await thisWindow.outerPosition();
        const size = await thisWindow.innerSize();

        for (const webview of webviewTabs) {
            const bounds = webview.label == currentWebviewTab.label ? util.toBounds(position, size) : webview.bounds;
            const isMaximized = webview.label == currentWebviewTab.label ? currentWebviewTab.isMaximized : webview.isMaximized;

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

        active = false;
        currentWebviewTab.label = "";
        webviewTabs = [];
    };

    const hideThis = async () => {
        const thisWindow = getCurrentWebviewWindow();
        await thisWindow.setPosition(new PhysicalPosition(OFF_SCREEN, OFF_SCREEN));
        await thisWindow.unmaximize();
        await thisWindow.hide();
    };

    const onKeyDown = (e: KeyboardEvent) => {
        if ((e.ctrlKey && BROWSER_SHORTCUT_KEYS.includes(e.key)) || SINGLE_BROWSER_SHORTCUT_KEYS.includes(e.key)) {
            e.preventDefault();
            e.stopPropagation();
        }
    };

    onMount(() => {
        ipc.receiveTauri("tauri://close-requested", beforeClose);
        ipc.receiveTauri("tauri://resize", onWindowSizeChanged);
        ipc.receiveTauri("tauri://resize", onResize);
        ipc.receiveTauri("tauri://focus", onFocus);
        ipc.receive("toggleMaximize", toggleMaximize);
        ipc.receive("minimize", minimize);
        ipc.receive("startTabMode", startTabMode);
        ipc.receive("endTabMode", endTabMode);
        ipc.receive("switchTab", switchTab);
        ipc.receive("updateTab", updateTab);
        ipc.receive("addTab", addTab);
        ipc.receive("closeTab", closeTab);
        ipc.receive("closeAll", closeAll);
        ipc.receive("closed", onAnotherWindowClosed);

        return () => {
            ipc.release();
        };
    });
</script>

<svelte:document ondragover={(e) => e.preventDefault()} onkeydown={onKeyDown} />

<div class="viewport"><div class="title-bar no-print"></div></div>
