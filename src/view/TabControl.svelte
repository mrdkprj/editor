<script lang="ts">
    import { flip } from "svelte/animate";
    import { tabState, tabs } from "./appStateReducer.svelte";
    import util from "../util";
    import { IPC } from "../ipc";

    let { label }: { label: string } = $props();

    // svelte-ignore state_referenced_locally
    const ipc = new IPC(label);
    let tab: HTMLDivElement;

    $effect(() => {
        tab.scrollLeft = tabs.scrollLeft;
    });

    const onCloseButtonMousedown = (e: MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
    };

    const closeTab = async (e: MouseEvent, label: string) => {
        e.preventDefault();
        e.stopPropagation();
        await ipc.sendTo(label, "tab_event", { name: "close" });
    };

    const onTabClick = (e: MouseEvent, selected: string) => {
        e.preventDefault();
        if (selected != label) {
            ipc.invoke("tab_request", { name: "select", data: selected });
        }
    };

    const onmousewheel = (e: WheelEvent) => {
        if (e.deltaY > 0) {
            tab.scrollBy(20, 0);
        } else {
            tab.scrollBy(-20, 0);
        }
        ipc.sendOthers("tab_event", { name: "scrolled", data: tab.scrollLeft });
    };

    const startDrag = async (e: DragEvent) => {
        if (util.isLinux()) {
            e.preventDefault();
        }

        if (!e.target || !(e.target instanceof HTMLElement)) return;
        tabState.dragging = true;
        tabState.startLabel = e.target.id;
        tabState.lastX = e.pageX;
    };

    const onDragOver = (e: DragEvent) => {
        e.preventDefault();
        if (util.isLinux()) return;
        drag(e);
    };

    /* On Linux, dragover never fires */
    const onMouseMove = (e: MouseEvent) => {
        if (util.isWin()) return;
        drag(e);
    };

    const drag = (e: MouseEvent | DragEvent) => {
        if (!tabState.dragging) return;

        if (e.pageX == tabState.lastX) return;
        const dargToRight = e.pageX > tabState.lastX;
        tabState.lastX = e.pageX;

        if (!e.target || !(e.target instanceof HTMLElement)) return;

        const id = e.target.getAttribute("data-drag-id") ?? "";

        if (!id) return;

        if (id == tabState.startLabel) return;

        if (!dargToRight && e.target.offsetLeft + e.target.clientWidth / 2 < e.pageX) {
            replace(tabState.startLabel, id);
        }

        if (dargToRight && e.target.offsetLeft + e.target.clientWidth / 2 > e.pageX) {
            replace(tabState.startLabel, id);
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
        tabState.willStartDrag = false;
        endDrag();
    };

    const onMouseUp = () => {
        tabState.willStartDrag = false;
        endDrag();
    };

    const endDrag = () => {
        if (!tabState.dragging) return;
        ipc.sendTo(label, "dragEnd", {});
        tabState.dragging = false;
        ipc.invoke("tab_request", { name: "reorder", data: tabs.webviews });
    };

    const onMouseDown = () => {
        tabState.willStartDrag = true;
    };

    const replace = (sourceId: string, targetId: string) => {
        const sourceIndex = tabs.webviews.findIndex((label) => label.label == sourceId);
        const source = tabs.webviews.splice(sourceIndex, 1)[0];

        const targetIndex = tabs.webviews.findIndex((label) => label.label == targetId);
        const shouldAppend = targetIndex >= sourceIndex;
        tabs.webviews.splice(shouldAppend ? targetIndex + 1 : targetIndex, 0, source);
    };
</script>

<div class="tab no-print" bind:this={tab} onwheel={onmousewheel}>
    {#each tabs.webviews as tab (tab.label)}
        <div
            id={tab.label}
            draggable="true"
            class="tablinks"
            data-drag-id={tab.label}
            class:tablinks-active={tab.label == label}
            onclick={(e) => onTabClick(e, tab.label)}
            onkeydown={() => {}}
            ondragstart={startDrag}
            ondragover={onDragOver}
            onmousemove={onMouseMove}
            onmouseup={onMouseUp}
            onmousedown={onMouseDown}
            ondrop={onDrop}
            role="button"
            tabindex="-1"
            animate:flip={{ duration: tabState.dragging ? 400 : 0 }}
        >
            <div class="tab-title" title={tab.title}>{tab.title}</div>
            <div class="tab-close-btn" onclick={(e) => closeTab(e, tab.label)} onmousedown={onCloseButtonMousedown} onkeydown={() => {}} role="button" tabindex="-1">
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
        border-bottom: 1px solid var(--tab-border);
        z-index: 1300;
        width: 100%;
        overflow: hidden;
        -webkit-app-region: no-drag;
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
        background-color: var(--tab-bg-color);
        color: var(--tab-color);
        font-size: 14px;
        border-right: 1px solid var(--tab-border);
        border-left: 1px solid var(--tab-border);
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
        flex-shrink: 0;
    }

    .tab-close-btn:hover {
        background-color: var(--tab-close-btn-hover-bg-color);
        color: var(--tab-close-btn-hover-color);
    }

    .tablinks:not(.tablinks-active):hover {
        background-color: var(--tab-hover-bg-color);
    }

    .tablinks-active {
        background-color: var(--tab-active-bg-color);
    }
</style>
