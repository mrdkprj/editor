<script lang="ts">
    import { flip } from "svelte/animate";
    import { tabs } from "./appStateReducer.svelte";

    let {
        currentLabel,
        switchTab,
        closeTab,
        onTabScroll,
        onTabMoved,
    }: { currentLabel: string; switchTab: (label: string) => void; closeTab: (label: string) => void; onTabScroll: (scrollLeft: number) => void; onTabMoved: () => void } = $props();

    type DragState = {
        startLabel: string;
        dragging: boolean;
        lastX: number;
    };
    const dragstate = $state<DragState>({ startLabel: "", dragging: false, lastX: 0 });
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

    const startDrag = async (e: DragEvent) => {
        if (!e.target || !(e.target instanceof HTMLElement)) return;
        dragstate.dragging = true;
        dragstate.startLabel = e.target.id;
        dragstate.lastX = e.pageX;
    };

    const onDragOver = (e: DragEvent) => {
        e.preventDefault();

        if (!dragstate.dragging) return;

        if (e.pageX == dragstate.lastX) return;
        const dargToRight = e.pageX > dragstate.lastX;
        dragstate.lastX = e.pageX;

        if (!e.target || !(e.target instanceof HTMLElement)) return;

        const id = e.target.getAttribute("data-drag-id") ?? "";

        if (!id) return;

        if (id == dragstate.startLabel) return;

        if (!dargToRight && e.target.offsetLeft + e.target.clientWidth / 2 < e.pageX) {
            replace(dragstate.startLabel, id);
        }

        if (dargToRight && e.target.offsetLeft + e.target.clientWidth / 2 > e.pageX) {
            replace(dragstate.startLabel, id);
        }

        if (tab.scrollWidth > tab.clientWidth) {
            if (e.target.offsetLeft + e.target.clientWidth > tab.clientWidth) {
                tab.scroll(e.target.offsetLeft + e.target.clientWidth - tab.clientWidth, 0);
            }

            if (tab.scrollLeft > e.target.offsetLeft) {
                tab.scroll(-20, 0);
            }
        }
    };

    const onDrop = () => {
        if (!dragstate.dragging) return;
        dragstate.dragging = false;
        onTabMoved();
    };

    const replace = (sourceId: string, targetId: string) => {
        const sourceIndex = tabs.webviews.findIndex((label) => label.label == sourceId);
        const source = tabs.webviews.splice(sourceIndex, 1)[0];

        const targetIndex = tabs.webviews.findIndex((label) => label.label == targetId);
        const shouldAppend = targetIndex >= sourceIndex;
        tabs.webviews.splice(shouldAppend ? targetIndex + 1 : targetIndex, 0, source);
    };
</script>

<div class="tab" bind:this={tab} onwheel={onmousewheel}>
    {#each tabs.webviews as tab (tab.label)}
        <div
            id={tab.label}
            draggable="true"
            class="tablinks"
            data-drag-id={tab.label}
            class:tablinks-active={tab.label == currentLabel}
            onclick={(e) => onTabClick(e, tab.label)}
            onkeydown={() => {}}
            ondragstart={startDrag}
            ondragover={onDragOver}
            ondrop={onDrop}
            role="button"
            tabindex="-1"
            animate:flip={{ duration: dragstate.dragging ? 400 : 0 }}
        >
            <div class="tab-title" title={tab.title}>{tab.title}</div>
            <div class="tab-close-btn" onclick={() => closeTab(tab.label)} onkeydown={() => {}} role="button" tabindex="-1">
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
