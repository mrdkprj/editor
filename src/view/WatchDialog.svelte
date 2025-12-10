<script lang="ts">
    import { onMount } from "svelte";
    import { handleKeyEvent } from "../constants";
    import { IPC } from "../ipc";
    import { dispatch } from "./appStateReducer.svelte";

    const ipc = new IPC("View");
    let doNotNotify = false;

    const onkeydown = (e: KeyboardEvent) => {
        if (e.key == "Escape") {
            close(false);
        }
    };

    const setKeyboardFocus = (node: HTMLDivElement) => {
        node.focus();
    };

    const close = (applyChange: boolean) => {
        ipc.sendTo("View", "watch_confirm_event", { applyChange, doNotNotify });
        dispatch({ type: "showWatchDialog", value: false });
        ipc.sendTo("View", "dialog", false);
    };

    onMount(() => {
        ipc.sendTo("View", "dialog", true);
    });
</script>

<div class="mp-dialog-overlay" {onkeydown} role="button" tabindex="-1" use:setKeyboardFocus>
    <div class="mp-dialog-container">
        <div class="mp-dialog-header">
            <div class="mp-dialog-close" onclick={() => close(false)} onkeydown={handleKeyEvent} role="button" tabindex="-1">&times;</div>
        </div>
        <div class="mp-dialog">
            <div class="mp-dialog-title-block">Apply Changes?</div>
            <div>File content has been changed. Do you apply the changes?</div>
            <div class="mp-dialog-item-block">
                <div class="mp-dialog-item">
                    <input type="checkbox" bind:checked={doNotNotify} id="doNotNotify" /><label for="doNotNotify">Do not notify again</label>
                </div>
            </div>
            <div class="mp-dialog-separator"></div>
            <div class="mp-dialog-action">
                <button class="mp-dialog-btn-lg" onclick={() => close(true)}>Yes</button>
                <button class="mp-dialog-btn-lg" onclick={() => close(false)}>No</button>
            </div>
        </div>
    </div>
</div>
