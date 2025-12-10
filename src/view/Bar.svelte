<script lang="ts">
    import { appState } from "./appStateReducer.svelte";
    import { handleKeyEvent, OS } from "../constants";
    import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { path } from "../path";
    import Menubar from "./Menubar.svelte";
    import icon from "../asset/icon.png";

    let { beforeClose, toggleMaximize }: { beforeClose: () => void; toggleMaximize: () => void } = $props();
    let disabled = $derived($appState.showGrepDialog || $appState.showGrepProgress);

    const onmousedown = (e: MouseEvent) => {
        if (disabled) {
            e.preventDefault();
        }
    };

    const minimize = async () => {
        await WebviewWindow.getCurrent().minimize();
    };
</script>

<div class="title-bar no-print" data-tauri-drag-region={navigator.userAgent.includes(OS.linux) ? true : null} class:bar-disabled={disabled}>
    <div class="icon-area" {onmousedown} role="button" tabindex="-1">
        <img src={icon} alt="" width="20" height="20" />
    </div>
    <div class="menu-bar-area" {onmousedown} role="button" tabindex="-1">
        <Menubar />
    </div>
    <div class="title" data-tauri-drag-region={navigator.userAgent.includes(OS.linux) ? true : null} {onmousedown} role="button" tabindex="-1">
        {$appState.fullPath ? path.basename($appState.fullPath) : $appState.mode == "grep" ? "Grep" : "Untitled"}{$appState.isDirty ? "*" : ""}
    </div>
    <div class="window-area">
        <div class="minimize" onclick={minimize} onkeydown={handleKeyEvent} role="button" tabindex="-1">&minus;</div>
        <div class="maximize" onclick={toggleMaximize} onkeydown={handleKeyEvent} role="button" tabindex="-1">
            <div class:minbtn={$appState.isMaximized} class:maxbtn={!$appState.isMaximized}></div>
        </div>
        <div class="close" onclick={beforeClose} onkeydown={handleKeyEvent} role="button" tabindex="-1">&times;</div>
    </div>
</div>

<style>
    .bar-disabled .menu-bar-area {
        pointer-events: none;
    }
</style>
