<script lang="ts">
    import { handleKeyEvent } from "../constants";
    import { dispatch, grepProgress } from "./appStateReducer.svelte";
    import { IPC } from "../ipc";
    import { onMount } from "svelte";

    let { label, abortGrep }: { label: string; abortGrep: () => Promise<void> } = $props();

    // svelte-ignore state_referenced_locally
    const ipc = new IPC(label);

    const onkeydown = (e: KeyboardEvent) => {
        e.preventDefault();
        e.stopPropagation();
        if (e.key == "Escape") {
            close();
        }
    };

    const setKeyboardFocus = (node: HTMLDivElement) => {
        node.focus();
    };

    const close = async () => {
        await abortGrep();
        dispatch({ type: "toggleDialog", value: { type: "progress", open: false } });
        ipc.sendTo(label, "dialog", false);
    };

    onMount(() => {
        ipc.sendTo(label, "dialog", true);
    });
</script>

<div class="mp-dialog-overlay" {onkeydown} role="button" tabindex="-1" use:setKeyboardFocus>
    <div class="mp-dialog-container">
        <div class="mp-dialog-header">
            <div class="mp-dialog-close" onclick={close} onkeydown={handleKeyEvent} role="button" tabindex="-1">&times;</div>
        </div>
        <div class="mp-dialog">
            <div class="mp-dialog-item-block">
                <div class="mp-dialog-title-block">Processing...</div>
                <div class="mp-dialog-item"><div class="mp-dialog-text">File: {grepProgress.file}</div></div>
                <div class="mp-dialog-item">{`${grepProgress.current}/${grepProgress.total}`}</div>
                <div class="mp-dialog-item">{`${grepProgress.matched} files found`}</div>
            </div>
            <div class="mp-dialog-separator"></div>
            <div class="mp-dialog-action">
                <button class="mp-dialog-btn-lg" onclick={close}>Abort</button>
            </div>
        </div>
    </div>
</div>
