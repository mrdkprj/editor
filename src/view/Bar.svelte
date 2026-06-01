<script lang="ts">
    import { appState, contentState } from "./appStateReducer.svelte";
    import { GREP, handleKeyEvent, UNTITLED } from "../constants";
    import path from "../path";
    import util from "../util";
    import Menubar from "./Menubar.svelte";
    import icon from "../asset/icon.png";
    import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

    let {
        label,
        openNewWindow,
        close,
        toggleMaximize,
        minimize,
    }: {
        label: string;
        openNewWindow: (filePath: string, grepRequest?: Mp.GrepRequest, position?: Mp.Position) => Promise<void>;
        close: () => void;
        toggleMaximize: () => void;
        minimize: () => void;
    } = $props();
    let disabled = $derived($appState.anyDialogOpened);

    const onmousedown = async (e: MouseEvent) => {
        if (disabled) {
            e.preventDefault();
        }
    };

    /* On Linux, webviews are attached to Main Window on tab mode */
    const dragGtkWindow = async (e: MouseEvent) => {
        if ($appState.visibleMenubarItem) return;

        if (!e.target || !(e.target instanceof HTMLElement)) return;

        if (e.target.classList.contains("title-bar") || e.target.classList.contains("title")) {
            const mainWindow = await WebviewWindow.getByLabel("Main");
            mainWindow?.startDragging();
        }
    };
</script>

{#if util.isWin()}
    <div class="title-bar no-print" class:bar-disabled={disabled}>
        <div class="icon-area" {onmousedown} onclick={() => openNewWindow("")} onkeydown={handleKeyEvent} role="button" tabindex="-1">
            <img src={icon} alt="" width="20" height="20" />
        </div>
        <div class="menu-bar-area" {onmousedown} role="button" tabindex="-1">
            <Menubar {label} />
        </div>
        <div class="title" title={contentState.fullPath} {onmousedown} role="button" tabindex="-1">
            {contentState.fullPath ? path.basename(contentState.fullPath) : $appState.mode == "grep" ? GREP : UNTITLED}{contentState.isDirty ? "*" : ""}
        </div>
        <div class="window-area">
            <div class="minimize" onclick={minimize} onkeydown={handleKeyEvent} role="button" tabindex="-1">&minus;</div>
            <div class="maximize" onclick={toggleMaximize} onkeydown={handleKeyEvent} role="button" tabindex="-1">
                <div class:minbtn={$appState.isMaximized} class:maxbtn={!$appState.isMaximized}></div>
            </div>
            <div class="close" onclick={close} onkeydown={handleKeyEvent} role="button" tabindex="-1">&times;</div>
        </div>
    </div>
{:else}
    <div class="title-bar no-print" onmousedown={dragGtkWindow} class:bar-disabled={disabled} role="button" tabindex="-1">
        <div class="icon-area" {onmousedown} onclick={() => openNewWindow("")} onkeydown={handleKeyEvent} role="button" tabindex="-1">
            <img src={icon} alt="" width="20" height="20" />
        </div>
        <div class="menu-bar-area" {onmousedown} role="button" tabindex="-1">
            <Menubar {label} />
        </div>
        <div class="title" title={contentState.fullPath} onmousedown={dragGtkWindow} role="button" tabindex="-1">
            {contentState.fullPath ? path.basename(contentState.fullPath) : $appState.mode == "grep" ? GREP : UNTITLED}{contentState.isDirty ? "*" : ""}
        </div>
        <div class="window-area">
            <div class="minimize" onclick={minimize} onkeydown={handleKeyEvent} role="button" tabindex="-1">&minus;</div>
            <div class="maximize" onclick={toggleMaximize} onkeydown={handleKeyEvent} role="button" tabindex="-1">
                <div class:minbtn={$appState.isMaximized} class:maxbtn={!$appState.isMaximized}></div>
            </div>
            <div class="close" onclick={close} onkeydown={handleKeyEvent} role="button" tabindex="-1">&times;</div>
        </div>
    </div>
{/if}

<style>
    .bar-disabled .menu-bar-area {
        pointer-events: none;
    }
</style>
