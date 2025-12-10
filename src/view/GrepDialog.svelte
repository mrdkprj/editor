<script lang="ts">
    import { handleKeyEvent } from "../constants";
    import { appState, dispatch } from "./appStateReducer.svelte";
    import { IPC } from "../ipc";
    import { onMount } from "svelte";

    let { showErrorMessage, executeGrep }: { executeGrep: (reqeust: Mp.GrepRequest) => void; showErrorMessage: (message: string) => Promise<void> } = $props();
    const ipc = new IPC("View");
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
        if (e.key == "Escape") {
            close();
        }

        if (e.key == "Enter") {
            runGrep();
        }
    };

    const setKeyboardFocus = (node: HTMLDivElement) => {
        node.focus();
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
        dispatch({ type: "showGrepDialog", value: false });
        ipc.sendTo("View", "dialog", false);
    };

    onMount(() => {
        ipc.sendTo("View", "dialog", true);
    });
</script>

<div class="mp-dialog-overlay" {onkeydown} role="button" tabindex="-1">
    <div class="mp-dialog-container">
        <div class="mp-dialog-header">
            <div class="mp-dialog-close" onclick={close} onkeydown={handleKeyEvent} role="button" tabindex="-1">&times;</div>
        </div>
        <div class="mp-dialog">
            <div class="mp-dialog-item-block">
                <div class="mp-dialog-title-block">Condition</div>
                <div class="mp-dialog-item"><input type="text" bind:value={request.condition} use:setKeyboardFocus /></div>
                <div class="mp-dialog-item"><input type="checkbox" id="byword" bind:checked={request.match_by_word} /><label for="byword">Matches on word boundaries</label></div>
                <div class="mp-dialog-item"><input type="checkbox" id="casesensitive" bind:checked={request.case_sensitive} /><label for="casesensitive">Case sensitive</label></div>
                <div class="mp-dialog-item"><input type="checkbox" id="regexp" bind:checked={request.regexp} /><label for="regexp">Use regular expression</label></div>
            </div>
            <div class="mp-dialog-item-block">
                <div class="mp-dialog-title-block">Location</div>
                <div class="mp-dialog-item"><input type="text" bind:value={request.start_directory} required /></div>
                <div class="mp-dialog-item"><input type="checkbox" id="recursive" bind:checked={request.recursive} /><label for="recursive">Include sub directories</label></div>
            </div>
            <div class="mp-dialog-item-block">
                <div class="mp-dialog-title-block">File Type</div>
                <div class="mp-dialog-item"><input type="text" bind:value={request.file_type} /></div>
            </div>
            <div class="mp-dialog-separator"></div>
            <div class="mp-dialog-action">
                <button class="mp-dialog-btn-lg" onclick={runGrep}>Grep</button>
                <button class="mp-dialog-btn-lg" onclick={close}>Cancel</button>
            </div>
        </div>
    </div>
</div>
