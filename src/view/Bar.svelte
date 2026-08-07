<script lang="ts">
    import { appState, contentState } from "./appStateReducer.svelte";
    import { GREP, handleKeyEvent, UNTITLED } from "../constants";
    import path from "../path";
    import Menubar from "./Menubar.svelte";
    import icon from "../asset/icon.png";
    import { IPCBase } from "../ipc";

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

    const ipc = new IPCBase();

    const onmousedown = async (e: MouseEvent) => {
        if (disabled) {
            e.preventDefault();
        }
    };

    const dragWindow = (e: MouseEvent) => {
        e.preventDefault();
        if ($appState.visibleMenubarItem) return;

        if (!e.target || !(e.target instanceof HTMLElement)) return;

        if (e.target.classList.contains("title-bar") || e.target.classList.contains("title")) {
            ipc.invoke("tab_request", { name: "startDrag" });
        }
    };
</script>

<div class="title-bar no-print" class:bar-disabled={disabled} onmousedown={dragWindow} onkeydown={handleKeyEvent} role="button" tabindex="-1">
    <div class="icon-area" {onmousedown} onclick={() => openNewWindow("")} onkeydown={handleKeyEvent} role="button" tabindex="-1">
        <img src={icon} alt="" width="20" height="20" />
    </div>
    <div class="menu-bar-area" {onmousedown} role="button" tabindex="-1">
        <Menubar {label} />
    </div>
    <div class="title" title={contentState.fullPath} onmousedown={dragWindow} role="button" tabindex="-1">
        {contentState.fullPath ? path.basename(contentState.fullPath) : contentState.mode == "grep" ? GREP : UNTITLED}{contentState.isDirty ? "*" : ""}
    </div>
    <div class="window-area">
        <div class="minimize" onclick={minimize} onkeydown={handleKeyEvent} role="button" tabindex="-1">&minus;</div>
        <div class="maximize" onclick={toggleMaximize} onkeydown={handleKeyEvent} role="button" tabindex="-1">
            <div class:minbtn={$appState.isMaximized} class:maxbtn={!$appState.isMaximized}></div>
        </div>
        <div class="close" onclick={close} onkeydown={handleKeyEvent} role="button" tabindex="-1">&times;</div>
    </div>
</div>

<!-- {#if util.isWin()}
    <div class="title-bar no-print" class:bar-disabled={disabled} onmousedown={(e) => e.preventDefault()} onkeydown={handleKeyEvent} role="button" tabindex="-1">
        <div class="icon-area" {onmousedown} onclick={() => openNewWindow("")} onkeydown={handleKeyEvent} role="button" tabindex="-1">
            <img src={icon} alt="" width="20" height="20" />
        </div>
        <div class="menu-bar-area" {onmousedown} role="button" tabindex="-1">
            <Menubar {label} />
        </div>
        <div class="title" title={contentState.fullPath} {onmousedown} role="button" tabindex="-1">
            {contentState.fullPath ? path.basename(contentState.fullPath) : contentState.mode == "grep" ? GREP : UNTITLED}{contentState.isDirty ? "*" : ""}
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
            {contentState.fullPath ? path.basename(contentState.fullPath) : contentState.mode == "grep" ? GREP : UNTITLED}{contentState.isDirty ? "*" : ""}
        </div>
        <div class="window-area">
            <div class="minimize" onclick={minimize} onkeydown={handleKeyEvent} role="button" tabindex="-1">&minus;</div>
            <div class="maximize" onclick={toggleMaximize} onkeydown={handleKeyEvent} role="button" tabindex="-1">
                <div class:minbtn={$appState.isMaximized} class:maxbtn={!$appState.isMaximized}></div>
            </div>
            <div class="close" onclick={close} onkeydown={handleKeyEvent} role="button" tabindex="-1">&times;</div>
        </div>
    </div>
{/if} -->

<style>
    .bar-disabled .menu-bar-area {
        pointer-events: none;
    }
</style>
