<script lang="ts">
    import { handleKeyEvent } from "../constants";
    import { dispatch, settings, textState, temporal, selectedPreference } from "./appStateReducer.svelte";
    import { IPC } from "../ipc";
    import { onMount } from "svelte";
    import { Colors, ColorTokens, dark_colors, lihgt_colors } from "../theme";

    const ipc = new IPC("View");
    const fontSizes = [10, 11, 12, 13, 14, 16, 18, 20];
    const indentSizes = [1, 2, 3, 4, 5, 6, 7, 8];
    let themeColors = $state(settings.theme == "dark" ? $state.snapshot(settings.color["dark"]) : $state.snapshot(settings.color["light"]));
    let preference = $state($state.snapshot(settings.preference[textState.textType]));

    const setKeyboardFocus = (node: HTMLDivElement) => {
        node.focus();
    };

    const onkeydown = (e: KeyboardEvent) => {
        if (e.key == "Escape") {
            close();
        }
    };

    const save = () => {
        let colorChanged = false;
        let preferenceChanged = false;

        if (JSON.stringify(settings.color[settings.theme]) != JSON.stringify(themeColors)) {
            settings.color[settings.theme] = themeColors;
            colorChanged = true;
        }

        if (JSON.stringify(settings.preference[textState.textType]) != JSON.stringify(preference)) {
            settings.preference[textState.textType] = preference;
            /* Prevent reactivity */
            temporal[textState.textType] = $state.snapshot(preference);
            preferenceChanged = true;
        }

        if (colorChanged || preferenceChanged) {
            ipc.sendTo("View", "settingChanged", colorChanged && preferenceChanged ? "both" : colorChanged ? "color" : "preference");
        }

        close();
    };

    const restore = () => {
        themeColors = settings.theme == "dark" ? dark_colors : lihgt_colors;
    };

    const close = () => {
        dispatch({ type: "showPreference", value: false });
        ipc.sendTo("View", "dialog", false);
    };

    onMount(() => {
        ipc.sendTo("View", "dialog", true);
    });
</script>

<div class="mp-dialog-overlay" role="button" tabindex="-1" {onkeydown} use:setKeyboardFocus>
    <div class="mp-dialog-container" style="min-height: 500px;position:relative;">
        <div class="mp-dialog-header">
            <div class="mp-dialog-close" onclick={close} onkeydown={handleKeyEvent} role="button" tabindex="-1">&times;</div>
        </div>
        <div class="mp-dialog preference">
            <div class="tab">
                <div class="tablinks" class:tablinks-active={selectedPreference.tab == "view"} onclick={() => (selectedPreference.tab = "view")} onkeydown={handleKeyEvent} role="button" tabindex="-1">
                    View
                </div>
                <div
                    class="tablinks"
                    class:tablinks-active={selectedPreference.tab == "color"}
                    onclick={() => (selectedPreference.tab = "color")}
                    onkeydown={handleKeyEvent}
                    role="button"
                    tabindex="-1"
                >
                    Color
                </div>
            </div>
            {#if selectedPreference.tab == "view"}
                <div class="mp-dialog-column">
                    <div class="mp-dialog-item-block">
                        <div class="mp-dialog-item"><label for="fontfamily">Font Family</label></div>
                        <div class="mp-dialog-item"><label for="fontsize">Font Size</label></div>
                        <div class="mp-dialog-item"><label for="linenumber">Show Line Number</label></div>
                        <div class="mp-dialog-item"><label for="autoindent">Auto Indent</label></div>
                        <div class="mp-dialog-item"><label for="wordwrap">Wordwrap</label></div>
                        <div class="mp-dialog-item"><label for="whitespace">Render White Sapce</label></div>
                        <div class="mp-dialog-item"><label for="highlight">Line Highlight</label></div>
                        <div class="mp-dialog-item"><label for="indent">Indent By Space</label></div>
                        <div class="mp-dialog-item"><label for="indentsize">Indent Size</label></div>
                    </div>

                    <div class="mp-dialog-item-block">
                        <div class="mp-dialog-item"><input type="text" id="fontfamily" bind:value={preference.fontFamily} /></div>
                        <div class="mp-dialog-item">
                            <select id="fontsize" bind:value={preference.fontSize}>
                                {#each fontSizes as size}
                                    <option value={size}>{size}</option>
                                {/each}
                            </select>
                        </div>
                        <div class="mp-dialog-item"><input type="checkbox" id="linenumber" bind:checked={preference.showLineNumber} /></div>
                        <div class="mp-dialog-item"><input type="checkbox" id="autoindent" bind:checked={preference.autoIndent} /></div>
                        <div class="mp-dialog-item"><input type="checkbox" id="wordwrap" bind:checked={preference.wordWrap} /></div>
                        <div class="mp-dialog-item">
                            <select id="whitespace" bind:value={preference.renderWhitespace}>
                                <option value="none">None</option>
                                <option value="selection">Selection</option>
                                <option value="all">All</option>
                            </select>
                        </div>
                        <div class="mp-dialog-item"><input type="checkbox" id="highlight" bind:checked={preference.lineHighlight} /></div>
                        <div class="mp-dialog-item"><input type="checkbox" id="indent" bind:checked={preference.indentBySpaces} /></div>
                        <div class="mp-dialog-item">
                            <select id="indentsize" bind:value={preference.indentSize}>
                                {#each indentSizes as size}
                                    <option value={size}>{size}</option>
                                {/each}
                            </select>
                        </div>
                    </div>
                </div>
            {/if}
            {#if selectedPreference.tab == "color"}
                <div class="mp-dialog-column">
                    <div class="mp-dialog-item-block">
                        {#each Object.keys(ColorTokens) as colorKey}
                            {#if colorKey in Colors}
                                {@const color = Colors[colorKey]}
                                <div class="mp-dialog-item">{color.label}</div>
                            {/if}
                        {/each}
                    </div>
                    <div class="mp-dialog-item-block">
                        {#each Object.keys(ColorTokens) as colorKey}
                            {#if colorKey in Colors}
                                {@const color = Colors[colorKey]}
                                <div class="mp-dialog-item">
                                    {#if color.group}
                                        <input type="color" title={themeColors[color.token]} bind:value={themeColors[color.token]} />
                                        <input type="color" title={themeColors[color.group]} bind:value={themeColors[color.group]} />
                                    {:else if color.background}
                                        <div class="empty-color"></div>
                                        <input type="color" title={themeColors[color.token]} bind:value={themeColors[color.token]} />
                                    {:else}
                                        <input type="color" title={themeColors[color.token]} bind:value={themeColors[color.token]} />
                                        <div class="empty-color"></div>
                                    {/if}
                                </div>
                            {/if}
                        {/each}
                    </div>
                </div>
            {/if}
            <div class="mp-dialog-footer">
                <div class="mp-dialog-separator"></div>
                <div class="mp-dialog-action">
                    <button class="mp-dialog-btn-lg" onclick={save}>Save</button>
                    {#if selectedPreference.tab == "color"}
                        <button class="mp-dialog-btn-lg" onclick={restore}>Restore Default</button>
                    {/if}
                    <button class="mp-dialog-btn-lg" onclick={close}>Cancel</button>
                </div>
            </div>
        </div>
    </div>
</div>

<style>
    .empty-color {
        width: 50px;
        height: 25px;
    }

    .tab {
        display: flex;
        border-bottom: 1px solid #ccc;
        margin-bottom: 15px;
    }

    .tablinks {
        padding: 5px 15px;
        margin: 0;
        outline: none;
        border: none;
        background: #ccc;
        border-top-left-radius: 4px;
        border-top-right-radius: 4px;
        background-color: var(--menu-bgcolor);
        color: var(--menu-color);
        font-size: 14px;
        border-right: 1px solid transparent;
    }

    .tablinks:not(.tablinks-active):hover {
        color: #ccc;
    }

    .tablinks-active {
        background-color: var(--menu-hover-color);
        border-right: 1px solid var(--tab-border);
    }

    .mp-dialog-column {
        display: flex;
        align-items: flex-start;
        height: 370px;
    }

    .mp-dialog-column .mp-dialog-item {
        height: 25px;
    }

    .mp-dialog-footer {
        width: 100%;
        left: 0;
        bottom: 24px;
    }

    .preference input[type="text"] {
        width: auto;
        padding: 0;
    }

    .mp-dialog-item-block {
        margin-right: 10px;
    }

    .preference label {
        white-space: nowrap;
        margin-right: 10px;
    }

    .preference select {
        height: 25px;
        width: 100%;
        line-height: 25px;
        text-indent: 5px;
        font-size: 14px;
        border-radius: 2px;
        border: 1px solid #ccc;
        white-space: nowrap;
    }

    .preference select:focus,
    .preference select:focus-visible {
        outline: 1px solid blue;
    }
</style>
