<script lang="ts">
    import { grepProgress } from "./appStateReducer.svelte";
    import { IPC } from "../ipc";
    import { onMount } from "svelte";
    import Dialog from "./Dialog.svelte";
    import { DIALOG_COLORS } from "../constants";

    let { label, abortGrep, closeDialog }: { label: string; abortGrep: () => Promise<void>; closeDialog: (type: Mp.DialogType) => void } = $props();

    // svelte-ignore state_referenced_locally
    const ipc = new IPC(label);

    const close = async () => {
        await abortGrep();
        closeDialog("progress");
        ipc.sendTo(label, "dialog", false);
    };

    onMount(() => {
        ipc.sendTo(label, "dialog", true);
    });
</script>

<Dialog minWidth={540} minHeight={200} overlayOffet={36} colors={DIALOG_COLORS} {close} focusOnMount={true}>
    {#snippet content()}
        <div class="dialog-item-block">
            <div class="dialog-title-block">Processing...</div>
            <div class="dialog-item"><div class="dialog-text">File: {grepProgress.file}</div></div>
            <div class="dialog-item">{`${grepProgress.current}/${grepProgress.total}`}</div>
            <div class="dialog-item">{`${grepProgress.matched} files found`}</div>
        </div>
    {/snippet}
    {#snippet action()}
        <button class="dialog-btn-lgw" onclick={close}>Abort</button>
    {/snippet}
</Dialog>

<style>
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
