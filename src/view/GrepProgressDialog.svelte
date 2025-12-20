<script lang="ts">
    import { handleKeyEvent } from "../constants";
    import { dispatch } from "./appStateReducer.svelte";
    import { IPC } from "../ipc";
    import { onMount } from "svelte";

    let { abortGrep }: { abortGrep: () => Promise<void> } = $props();

    const ipc = new IPC("View");

    let file = $state("");
    let total = $state(0);
    let current = $state(0);

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
        dispatch({ type: "showGrepProgress", value: false });
    };

    const onProgress = (e: Mp.GrepProgress) => {
        file = e.processing;
        current = e.current;
        total = e.total;
    };

    onMount(() => {
        ipc.receive("grep_progress", onProgress);

        return () => {
            ipc.release();
        };
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
                <div class="mp-dialog-item"><div class="mp-dialog-text">File: {file}</div></div>
                <div class="mp-dialog-item">{`${current}/${total}`}</div>
            </div>
            <div class="mp-dialog-separator"></div>
            <div class="mp-dialog-action">
                <button class="mp-dialog-btn-lg" onclick={close}>Abort</button>
            </div>
        </div>
    </div>
</div>
