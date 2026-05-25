<script lang="ts">
    import { appState, contentState } from "./appStateReducer.svelte";
    import { GREP, handleKeyEvent, OS, UNTITLED } from "../constants";
    import path from "../path";
    import Menubar from "./Menubar.svelte";
    import icon from "../asset/icon.png";

    let {
        openNewWindow,
        close,
        toggleMaximize,
        minimize,
    }: { openNewWindow: (filePath: string, grepRequest?: Mp.GrepRequest, position?: Mp.Position) => Promise<void>; close: () => void; toggleMaximize: () => void; minimize: () => void } = $props();
    let disabled = $derived($appState.showGrepDialog || $appState.showGrepProgress || $appState.showPreference || $appState.showWatchDialog);

    const onmousedown = (e: MouseEvent) => {
        if (disabled) {
            e.preventDefault();
        }
    };
</script>

<div class="title-bar no-print" data-tauri-drag-region={navigator.userAgent.includes(OS.linux) ? true : null} class:bar-disabled={disabled}>
    <div class="icon-area" {onmousedown} onclick={() => openNewWindow("")} onkeydown={handleKeyEvent} role="button" tabindex="-1">
        <img src={icon} alt="" width="20" height="20" />
    </div>
    <div class="menu-bar-area" {onmousedown} role="button" tabindex="-1">
        <Menubar />
    </div>
    <div class="title" title={contentState.fullPath} data-tauri-drag-region={navigator.userAgent.includes(OS.linux) ? true : null} {onmousedown} role="button" tabindex="-1">
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

<style>
    .bar-disabled .menu-bar-area {
        pointer-events: none;
    }
</style>
