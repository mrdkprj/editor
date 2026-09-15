<script lang="ts">
    import { flip } from "svelte/animate";
    import { tabState } from "./appStateReducer.svelte";
    import util from "../util";
    import { IPC } from "../ipc";
    import { onMount } from "svelte";

    let { label }: { label: string } = $props();

    // svelte-ignore state_referenced_locally
    const ipc = new IPC(label);

    type DragState = {
        startLabel: string;
        lastX: number;
        needsDetach: boolean;
        dragEvent: Tab.StartDragEvent | null;
        insideWindow: boolean;
        dragCounter: number;
    };
    const dragState: DragState = { startLabel: "", lastX: 0, needsDetach: true, dragEvent: null, insideWindow: true, dragCounter: 0 };

    let tab: HTMLDivElement;

    $effect(() => {
        tab.scrollLeft = tabState.scrollLeft;
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
            e.dataTransfer?.setData("text/plain", "dummy");
        }

        if (!e.target || !(e.target instanceof HTMLElement)) return;

        dragState.needsDetach = true;
        dragState.dragEvent = null;
        dragState.dragCounter = 0;
        dragState.insideWindow = true;
        dragState.startLabel = e.target.id;
        dragState.lastX = e.pageX;
        await ipc.sendOthers("startDrag", { initiator: label, target: e.target.id });
        tabState.dragging = true;
    };

    const onDragOver = (e: DragEvent) => {
        e.preventDefault();
        drag(e);
    };

    const drag = (e: MouseEvent | DragEvent) => {
        if (!tabState.dragging) return;

        if (e.pageX == dragState.lastX) return;
        const dargToRight = e.pageX > dragState.lastX;
        dragState.lastX = e.pageX;

        if (!e.target || !(e.target instanceof HTMLElement)) return;

        const id = e.target.getAttribute("data-drag-id") ?? "";

        if (!id) return;

        if (id == dragState.startLabel) return;

        if (!dargToRight && e.target.offsetLeft + e.target.clientWidth / 2 < e.pageX) {
            replace(dragState.startLabel, id);
        }

        if (dargToRight && e.target.offsetLeft + e.target.clientWidth / 2 > e.pageX) {
            replace(dragState.startLabel, id);
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

    const onDrop = (e: DragEvent) => {
        tabState.willStartDrag = false;
        endDrag(e);
    };

    const onMouseUp = (e: MouseEvent) => {
        tabState.willStartDrag = false;
        endDrag(e);
    };

    const endDrag = (e: MouseEvent | DragEvent) => {
        if (dragState.dragEvent) {
            return attachTab(e);
        }

        if (!tabState.dragging) return;

        ipc.sendTo(label, "restoreFocus", null);
        tabState.dragging = false;
        ipc.invoke("tab_request", { name: "reorder", data: tabState.tabs });
    };

    const attachTab = (e: MouseEvent | DragEvent) => {
        /* Send only to the initiator for speed */
        ipc.sendTo(dragState.dragEvent!.initiator, "dropHandled", null);

        const element = document.elementFromPoint(e.clientX, e.clientY);
        const target = element?.classList.contains("tab-title") ? element.parentElement : element;

        let attach_target = null;
        let attach_before = false;
        if (target && target.hasAttribute("data-drag-id")) {
            const rect = target.getBoundingClientRect();
            const mid = rect.left + rect.width / 2;
            attach_before = e.clientX <= mid;
            attach_target = target.id;
        }
        ipc.invoke("tab_request", { name: "attach", data: { from: dragState.dragEvent!.target, to: label, attach_target, attach_before } });
        dragState.dragEvent = null;
    };

    const onDragEnd = (e: DragEvent) => {
        let effect = e.dataTransfer?.dropEffect;

        // const isOutside = e.clientX <= 0 || e.clientY <= 0 || e.clientX >= window.innerWidth || e.clientY >= window.innerHeight;

        if (!dragState.insideWindow) {
            switch (effect) {
                case "none":
                    ipc.invoke("tab_request", { name: "detach", data: { label: dragState.startLabel, offset_x: e.screenX, offset_y: e.screenY } });
                    break;

                case "move":
                case "copy":
                    setTimeout(() => {
                        console.log(dragState.needsDetach);
                        if (dragState.needsDetach) {
                            ipc.invoke("tab_request", { name: "detach", data: { label: dragState.startLabel, offset_x: e.screenX, offset_y: e.screenY } });
                        }
                    }, 50);
                    break;
            }
        }

        ipc.sendOthers("endDrag", null);
        tabState.dragging = false;
        dragState.dragCounter = 0;
        dragState.insideWindow = true;
    };

    const onDragEnter = () => {
        dragState.dragCounter++;
        dragState.insideWindow = true;
    };

    const onDragLeave = () => {
        dragState.dragCounter--;
        if (dragState.dragCounter == 0) {
            dragState.insideWindow = false;
        }
    };

    const onMouseDown = () => {
        tabState.willStartDrag = true;
    };

    const replace = (sourceId: string, targetId: string) => {
        const sourceIndex = tabState.tabs.findIndex((label) => label.label == sourceId);
        const source = tabState.tabs.splice(sourceIndex, 1)[0];

        const targetIndex = tabState.tabs.findIndex((tab) => tab.label == targetId);
        const shouldAppend = targetIndex >= sourceIndex;
        tabState.tabs.splice(shouldAppend ? targetIndex + 1 : targetIndex, 0, source);
    };

    onMount(() => {
        ipc.receive("dropHandled", () => (dragState.needsDetach = false));
        ipc.receive("startDrag", (e: Tab.StartDragEvent) => (dragState.dragEvent = e));
        ipc.receive("endDrag", () => (dragState.dragEvent = null));
        return () => {
            ipc.release();
        };
    });
</script>

<svelte:window ondragenter={onDragEnter} ondragleave={onDragLeave} />
<div class="tab no-print" bind:this={tab} onwheel={onmousewheel} ondrop={onDrop} role="button" tabindex="-1">
    {#each tabState.tabs as tab (tab.label)}
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
            onmouseup={onMouseUp}
            onmousedown={onMouseDown}
            ondragend={onDragEnd}
            ondrop={onDrop}
            role="button"
            tabindex="-1"
            animate:flip={{ duration: tabState.dragging ? 400 : 0 }}
        >
            <div class="tab-title" title={tab.title}>{tab.title}</div>
            <div class="tab-close-btn" onclick={(e) => closeTab(e, tab.label)} onmousedown={onCloseButtonMousedown} onkeydown={() => {}} role="button" tabindex="-1">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
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
