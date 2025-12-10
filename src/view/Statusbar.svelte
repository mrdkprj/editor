<script lang="ts">
    import { appState, textState, settings } from "./appStateReducer.svelte";
</script>

<div class="status-bar no-print">
    <div>Ln {$appState.cusorPosition.line}, Col {$appState.cusorPosition.column}</div>
    <div>Spaces: {settings.preference[textState.textType].indentSize}</div>
    <div>{textState.encoding}</div>
    <div>{$appState.lineEnding}</div>
    <div>{$appState.language}</div>
</div>

<style>
    .status-bar {
        height: 25px;
        display: flex;
        flex-direction: row;
        align-items: center;
        justify-content: flex-end;
        font-size: 12px;
        font-family: --var(--font);
    }

    .status-bar div {
        margin-right: 20px;
    }
</style>
