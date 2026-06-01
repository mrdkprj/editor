<script lang="ts">
    import { tabs } from "./appStateReducer.svelte";

    let { currentLabel, switchTab, closeTab, onTabScroll }: { currentLabel: string; switchTab: (label: string) => void; closeTab: () => void; onTabScroll: (scrollLeft: number) => void } = $props();

    let tab: HTMLDivElement;

    $effect(() => {
        tab.scrollLeft = tabs.scrollLeft;
    });

    const onTabClick = (e: MouseEvent, selected: string) => {
        e.preventDefault();
        if (selected != currentLabel) {
            switchTab(selected);
        }
    };

    const onmousewheel = (e: WheelEvent) => {
        if (e.deltaY > 0) {
            tab.scrollBy(20, 0);
        } else {
            tab.scrollBy(-20, 0);
        }
        onTabScroll(tab.scrollLeft);
    };
</script>

<div class="tab" bind:this={tab} onwheel={onmousewheel}>
    {#each Object.entries(tabs.webviews) as [label, webviewTitle]}
        <div class="tablinks" class:tablinks-active={label == currentLabel} onclick={(e) => onTabClick(e, label)} onkeydown={() => {}} role="button" tabindex="-1">
            <div class="tab-title" title={webviewTitle.title}>{webviewTitle.title}</div>
            <div class="tab-close-btn" onclick={closeTab} onkeydown={() => {}} role="button" tabindex="-1">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-x" viewBox="0 0 16 16">
                    <path
                        d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708"
                    />
                </svg>
            </div>
        </div>
    {/each}
</div>

<style>
    .tab {
        display: flex;
        border-bottom: 1px solid #ccc;
        z-index: 1300;
        width: 100%;
        overflow: hidden;
    }

    .tablinks {
        user-select: none;
        padding: 5px;
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
        flex-shrink: 0;
        max-width: 300px;
        min-width: 100px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        position: relative;
        display: flex;
        justify-content: center;
        cursor: pointer;
    }

    .tab-title {
        user-select: none;
        flex: 1 1 auto;
        margin-left: 5px;
        overflow: hidden;
        height: 100%;
        text-overflow: ellipsis;
        white-space: nowrap;
        text-align: center;
    }

    .tab-close-btn {
        user-select: none;
        display: flex;
        justify-content: center;
        align-items: center;
        width: 20px;
        height: 20px;
        margin-left: 5px;
        border-radius: 8px;
    }

    .tab-close-btn:hover {
        background-color: #ccc;
        color: black;
    }

    .tablinks:not(.tablinks-active):hover {
        color: #ccc;
    }

    .tablinks-active {
        background-color: var(--menu-hover-color);
        border-right: 1px solid var(--tab-border);
    }
</style>
