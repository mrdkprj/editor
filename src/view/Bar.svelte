<script lang="ts">
    import { appState, contentState } from "./appStateReducer.svelte";
    import { GREP, handleKeyEvent, UNTITLED } from "../constants";
    import path from "../path";
    import Menubar from "./Menubar.svelte";
    import icon from "../asset/icon.png";
    import { IPCBase } from "../ipc";
    import util from "../util";

    const ipc = new IPCBase();

    let {
        label,
        close,
        toggleMaximize,
        minimize,
    }: {
        label: string;
        close: () => void;
        toggleMaximize: () => void;
        minimize: () => void;
    } = $props();

    let disabled = $derived($appState.anyDialogOpened);
    let mayDragWindow = false;

    const onmousedown = async (e: MouseEvent) => {
        if (disabled) {
            e.preventDefault();
        }

        if (!e.target || !(e.target instanceof HTMLElement)) return;
        if (e.target.classList.contains("drag-region")) {
            mayDragWindow = true;
        }
    };

    const dragWindow = (e: DragEvent) => {
        e.preventDefault();
        if (mayDragWindow) {
            ipc.invoke("tab_request", { name: "startDrag" });
            mayDragWindow = false;
        }
    };

    const onmouseup = () => {
        mayDragWindow = false;
    };

    const onDblClick = (e: MouseEvent) => {
        if (!e.target || !(e.target instanceof HTMLElement)) return;
        if (e.target.classList.contains("drag-region")) {
            toggleMaximize();
        }
    };
</script>

<div
    class="title-bar no-print"
    class:bar-disabled={disabled}
    class:drag-region={util.isLinux()}
    draggable="true"
    {onmousedown}
    {onmouseup}
    ondblclick={onDblClick}
    ondragstart={dragWindow}
    onkeydown={handleKeyEvent}
    role="button"
    tabindex="-1"
>
    <div class="icon-area" class:drag-region={util.isLinux()} {onmousedown} {onmouseup} onkeydown={handleKeyEvent} role="button" tabindex="-1">
        <img src={icon} alt="" width="20" height="20" />
    </div>
    <div class="menu-bar-area" {onmousedown} {onmouseup} role="button" tabindex="-1">
        <Menubar {label} />
    </div>
    <div class="title" class:drag-region={util.isLinux()} title={contentState.fullPath} {onmousedown} {onmouseup} ondragstart={dragWindow} draggable="true" role="button" tabindex="-1">
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

<style>
    .bar-disabled .menu-bar-area {
        pointer-events: none;
    }
</style>
