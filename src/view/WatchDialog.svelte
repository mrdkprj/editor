<script lang="ts">
    import { onMount } from "svelte";
    import { IPC } from "../ipc";
    import Dialog from "./Dialog.svelte";
    import { DIALOG_COLORS } from "../constants";

    let { label, closeDialog }: { label: string; closeDialog: (type: Mp.DialogType) => void } = $props();

    // svelte-ignore state_referenced_locally
    const ipc = new IPC(label);
    let doNotNotify = $state(false);

    const onCloseButtonClick = () => {
        close(false);
    };

    const close = (applyChange: boolean) => {
        ipc.sendTo(label, "watch_confirm_event", { applyChange, doNotNotify });
        closeDialog("watch");
        ipc.sendTo(label, "dialog", false);
    };

    onMount(() => {
        ipc.sendTo(label, "dialog", true);
    });
</script>

<Dialog minWidth={540} minHeight={200} overlayOffet={36} colors={DIALOG_COLORS} close={onCloseButtonClick} focusOnMount={true}>
    {#snippet content()}
        <div class="dialog">
            <div class="dialog-title-block">Apply Changes?</div>
            <div>File content has been changed. Do you apply the changes?</div>
            <div class="dialog-item-block">
                <div class="dialog-item">
                    <input type="checkbox" bind:checked={doNotNotify} id="doNotNotify" /><label for="doNotNotify">Do not notify again</label>
                </div>
            </div>
            <div class="dialog-separator"></div>
            <div class="dialog-action">
                <button class="dialog-btn-lgw" onclick={() => close(true)}>Yes</button>
                <button class="dialog-btn-lgw" onclick={() => close(false)}>No</button>
            </div>
        </div>
    {/snippet}
</Dialog>

<style>
    .dialog input[type="checkbox"] {
        margin: 5px;
    }

    .dialog-btn-lgw {
        background-color: var(--button-bgcolor);
        color: var(--button-color);
        border: 1px solid var(--dialog-border-color);
        border-radius: 4px;
    }

    .dialog-btn-lgw:hover {
        background-color: var(--button-hover-color);
    }
</style>
