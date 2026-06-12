<script lang="ts">
    import { onMount } from "svelte";
    import { getCurrentWebviewWindow, WebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { Webview } from "@tauri-apps/api/webview";
    import { PhysicalPosition } from "@tauri-apps/api/dpi";
    import { IPC } from "../ipc";
    import util from "../util";
    import { BROWSER_SHORTCUT_KEYS, MAIN_LABEL, OS, SINGLE_BROWSER_SHORTCUT_KEYS } from "../constants";

    type ResizeEvent = {
        width: number;
        height: number;
    };

    const ipc = new IPC(MAIN_LABEL);

    const OFF_SCREEN = -30000;
    const isLinux = navigator.userAgent.includes(OS.linux);

    let windowState: Mp.WebviewTab = { label: "", title: "", path: "", bounds: { x: 0, y: 0, width: 0, height: 0 }, isMaximized: false };
    let active = false;
    let currentTabLabel = "";
    let webviewTabs: Mp.WebviewTab[] = [];
    let closingLabelSequence: string[] = [];
    let allTabClosing = false;

    const toggleMaximize = async () => {
        const thisWindow = getCurrentWebviewWindow();
        const isMaximized = await thisWindow.isMaximized();
        if (isMaximized) {
            thisWindow.unmaximize();
            if (!isLinux) {
                thisWindow.setPosition(util.toPhysicalPosition(windowState.bounds));
            }
        } else {
            const position = await thisWindow.innerPosition();
            const size = await thisWindow.innerSize();
            windowState.bounds = util.toBounds(position, size);
            await thisWindow.maximize();
        }

        windowState.isMaximized = !isMaximized;

        await ipc.sendOthers("tabWindowSizeChange", !isMaximized);
    };

    const minimize = async () => {
        const thisWindow = getCurrentWebviewWindow();
        const position = await thisWindow.innerPosition();
        const size = await thisWindow.innerSize();
        windowState.bounds = util.toBounds(position, size);
        await thisWindow.minimize();
    };

    const onWindowSizeChanged = async (e: ResizeEvent) => {
        if (!active) return;

        await updateWindowState(e);
        await ipc.sendOthers("tabWindowSizeChange", windowState.isMaximized);

        if (isLinux) return;

        /* On Windows, windows other than the current Windowresize are not autimatically resized */
        for (const tab of webviewTabs) {
            const webviewWindow = await WebviewWindow.getByLabel(tab.label);
            await webviewWindow?.setSize(util.toPhysicalSize(windowState.bounds));
        }
    };

    const updateWindowState = async (e: ResizeEvent) => {
        const isMaximized = await getCurrentWebviewWindow().isMaximized();
        windowState.isMaximized = isMaximized;
        windowState.bounds.width = e.width;
        windowState.bounds.height = e.height;
    };

    const beforeClose = async () => {
        const thisWindow = getCurrentWebviewWindow();
        const isMinimized = await thisWindow.isMinimized();
        if (isMinimized) {
            await thisWindow.unminimize();
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
        ipc.sendTo(closedLabel, "destory", windowState);
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
            if (e.webviewTitle.label == currentTabLabel) {
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
            /* First place window off-screen and change size*/
            await webviewWindow.setPosition(new PhysicalPosition(OFF_SCREEN, OFF_SCREEN));
            await webviewWindow.setSize(util.toPhysicalSize(windowState.bounds));
            /* Then show window with animation if not visible */
            const visible = await webviewWindow.isVisible();
            if (!visible) {
                await webviewWindow.show();
            }
        }

        ipc.sendTo(title.label, "tabWindowSizeChange", windowState.isMaximized);
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
            await switchTab(label);
        }, 50);
    };

    const switchTab = async (activeTabLabel: string) => {
        if (activeTabLabel == currentTabLabel) return;

        if (isLinux) {
            const toActive = await Webview.getByLabel(activeTabLabel);
            await toActive?.show();
            /* Keep webview visible and only change z-index */
            await ipc.invoke("bring_to_front", activeTabLabel);
        } else {
            // const size = await getCurrentWebviewWindow().size();
            const toActive = await WebviewWindow.getByLabel(activeTabLabel);
            await toActive?.setSize(util.toPhysicalSize(windowState.bounds));
            await toActive?.setPosition(new PhysicalPosition(-8, 0));
        }

        if (currentTabLabel) {
            if (!isLinux) {
                const toInactive = await WebviewWindow.getByLabel(currentTabLabel);
                toInactive?.setPosition(new PhysicalPosition(OFF_SCREEN, OFF_SCREEN));
            }
        }

        currentTabLabel = activeTabLabel;

        await activateWebview();
    };

    const activateWebview = async () => {
        /* Make the view restore focus on its editor */
        await ipc.sendTo(currentTabLabel, "tabActivated", {});

        /*
            On Windows, focus must be set on active child window.
            But focusing on a child window causes problems and app switching like ALT+Tab does not work.
            So place focus on the active child window's webview instead.
         */
        const activatedWebvivew = await Webview.getByLabel(currentTabLabel);
        await activatedWebvivew?.setFocus();
    };

    const onFocus = () => {
        if (currentTabLabel) {
            activateWebview();
        }
    };

    const startTabMode = async (initial: string) => {
        webviewTabs = [];
        currentTabLabel = "";

        const initiatedWindow = await WebviewWindow.getByLabel(initial);
        if (!initiatedWindow) return;
        const isMaximized = await initiatedWindow.isMaximized();
        const position = await initiatedWindow.innerPosition();
        const size = await initiatedWindow.innerSize();
        windowState.bounds = util.toBounds(position, size);
        windowState.isMaximized = isMaximized;

        const thisWindow = getCurrentWebviewWindow();
        await thisWindow.setSize(util.toPhysicalSize(windowState.bounds));
        await thisWindow.setPosition(util.toPhysicalPosition(windowState.bounds));
        const openedWebviews = await ipc.invoke("get_webview_labels", undefined);
        const sortedLabels = Object.keys(openedWebviews.webviews).toSorted();
        for (const label of sortedLabels) {
            await pushWebiew(openedWebviews.webviews[label]);
        }

        await ipc.invoke("to_child_window", sortedLabels);
        await ipc.sendOthers("enterTabMode", webviewTabs);

        await switchTab(initial);

        active = true;

        /* On Linux(Wayland), if previously maximized, window is forcibly maximized */
        await thisWindow.unmaximize();
        await thisWindow.show();
        if (windowState.isMaximized) {
            await thisWindow.maximize();
        }
    };

    const endTabMode = async () => {
        for (const tab of webviewTabs) {
            await ipc.invoke("restore_webview", tab.label);

            const webiewWindow = await WebviewWindow.getByLabel(tab.label);
            if (isLinux) {
                /* On Linux, WebviewWindow is hidden. So show it first */
                await webiewWindow?.show();
            }

            if (tab.isMaximized) {
                await webiewWindow?.maximize();
            } else {
                await webiewWindow?.unmaximize();
                await webiewWindow?.setPosition(util.toPhysicalPosition(tab.bounds));
                await webiewWindow?.setSize(util.toPhysicalSize(tab.bounds));
            }

            await ipc.sendTo(tab.label, "exitTabMode", tab.isMaximized);
        }

        await hideThis();

        active = false;
        currentTabLabel = "";
        webviewTabs = [];
    };

    const hideThis = async () => {
        const thisWindow = getCurrentWebviewWindow();
        const isMinimized = await thisWindow.isMinimized();
        if (isMinimized) {
            await thisWindow.unminimize();
        }
        await thisWindow.unmaximize();
        await thisWindow.setPosition(new PhysicalPosition(OFF_SCREEN, OFF_SCREEN));
        await thisWindow.setSize(util.toPhysicalSize(windowState.bounds));
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
