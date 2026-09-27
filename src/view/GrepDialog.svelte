<script lang="ts">
    import { DIALOG_COLORS } from "../constants";
    import { appState, dispatch } from "./appStateReducer.svelte";
    import { IPC } from "../ipc";
    import { onMount } from "svelte";
    import Dialog from "./Dialog.svelte";

    let { label, showErrorMessage, executeGrep }: { label: string; executeGrep: (reqeust: Mp.GrepRequest) => void; showErrorMessage: (message: string) => Promise<void> } = $props();

    // svelte-ignore state_referenced_locally
    const ipc = new IPC(label);
    let request: Mp.GrepRequest = $state({
        condition: $appState.grepRequest?.condition,
        start_directory: $appState.grepRequest?.start_directory,
        file_type: $appState.grepRequest?.file_type,
        match_by_word: $appState.grepRequest?.match_by_word,
        case_sensitive: $appState.grepRequest?.case_sensitive,
        regexp: $appState.grepRequest?.regexp,
        recursive: $appState.grepRequest?.recursive,
    });

    const onkeydown = (e: KeyboardEvent) => {
        if (e.key == "Enter") {
            runGrep();
        }
    };

    const setKeyboardFocus = (node: HTMLDivElement) => {
        node.focus();
    };

    const selectFolder = async () => {
        const result = await ipc.invoke("show_folder_dialog", { dialog_type: "ask", message: "", default_path: request.start_directory });
        if (result) {
            request.start_directory = result;
        }
    };

    const runGrep = async () => {
        if (!request.condition) {
            return await showErrorMessage("Condition is empty");
        }

        if (!request.start_directory) {
            return await showErrorMessage("Location is empty");
        }

        executeGrep(request);
        close();
    };

    const close = () => {
        dispatch({ type: "toggleDialog", value: { type: "grep", open: false } });
        ipc.sendTo(label, "dialog", false);
    };

    onMount(() => {
        ipc.sendTo(label, "dialog", true);
    });
</script>

<Dialog minWidth={540} minHeight={200} overlayOffet={36} colors={DIALOG_COLORS} {close} {onkeydown}>
    {#snippet content()}
        <div class="dialog-item-block">
            <div class="dialog-title-block">Condition</div>
            <div class="dialog-item"><input type="text" bind:value={request.condition} use:setKeyboardFocus /></div>
            <div class="dialog-item"><input type="checkbox" id="byword" bind:checked={request.match_by_word} /><label for="byword">Matches on word boundaries</label></div>
            <div class="dialog-item"><input type="checkbox" id="casesensitive" bind:checked={request.case_sensitive} /><label for="casesensitive">Case sensitive</label></div>
            <div class="dialog-item"><input type="checkbox" id="regexp" bind:checked={request.regexp} /><label for="regexp">Use regular expression</label></div>
        </div>
        <div class="dialog-item-block">
            <div class="dialog-title-block">Location</div>
            <div class="dialog-item"><input type="text" bind:value={request.start_directory} required /><button class="select-folder-button" onclick={selectFolder}>...</button></div>
            <div class="dialog-item"><input type="checkbox" id="recursive" bind:checked={request.recursive} /><label for="recursive">Include sub directories</label></div>
        </div>
        <div class="dialog-item-block">
            <div class="dialog-title-block">File Type</div>
            <div class="dialog-item"><input type="text" bind:value={request.file_type} /></div>
        </div>
    {/snippet}
    {#snippet action()}
        <button class="dialog-btn-lgw" onclick={runGrep}>Grep</button>
        <button class="dialog-btn-lgw" onclick={close}>Cancel</button>
    {/snippet}
</Dialog>

<style>
    .select-folder-button {
        width: 22px;
        vertical-align: bottom;
        text-align: center;
        line-height: 22px;
    }

    .dialog-item input[type="text"] {
        width: 100%;
        line-height: 22px;
        text-indent: 5px;
        font-size: 14px;
        border-radius: 2px;
        border: 1px solid #ccc;
        padding: 4px;
    }

    .dialog-item input[type="text"]:focus,
    .dialog-item input[type="text"]:focus-visible {
        outline: 1px solid var(--input-focus-outline);
    }

    .dialog-item input[type="checkbox"] {
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
