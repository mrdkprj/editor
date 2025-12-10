<script lang="ts">
    import { handleKeyEvent } from "../constants";
    import Menu from "./Menu.svelte";
    import { IPC } from "../ipc";
    import { onMount } from "svelte";
    import { appState, dispatch } from "./appStateReducer.svelte";

    let { items, submenu, id }: { items: Mp.MenuItem[]; submenu: boolean; id?: string } = $props();

    const ipc = new IPC("View");
    let canvas: HTMLCanvasElement;
    let width = $state(0);
    let hasCheckmark = $state(false);

    const onmousedown = (e: MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
    };

    const ignore = (e: MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        dispatch({ type: "openingMenu", value: true });
    };

    const emit = (e: MouseEvent) => {
        if (!e.target || !(e.target instanceof HTMLElement)) return;
        e.preventDefault();
        e.stopPropagation();

        if (e.target.classList.contains("submenu-container")) {
            dispatch({ type: "openingMenu", value: true });
            return;
        }

        if (e.target.classList.contains("radio")) {
            toggleRadio(e.target);
        } else if (e.target.classList.contains("checkbox-menu")) {
            toggleCheck(e.target);
        }

        ipc.sendTo("View", "contextmenu_event", { id: e.target.id as keyof Mp.MainContextMenuSubTypeMap, value: e.target.getAttribute("data-value") ?? undefined });
    };

    const toggleCheck = (target: HTMLElement) => {
        const item = items.find((item) => item.id == target.id);
        if (item) {
            item.checked = !item.checked;
        }
    };

    const toggleRadio = (target: HTMLElement) => {
        const group = target.getAttribute("data-group");
        if (!group) return;

        const radioItems = items.filter((item) => {
            return item.id == group;
        });
        if (!radioItems.length) return;

        const value = target.getAttribute("data-value");
        radioItems.forEach((item) => {
            if (item.value == value) {
                item.checked = true;
            } else {
                item.checked = false;
            }
        });
    };

    const onmouseenter = (e: MouseEvent) => {
        if (!e.target || !(e.target instanceof HTMLElement)) return;
        dispatch({ type: "hoverMenuItemGroup", value: e.target.getAttribute("data-submenu") ?? "" });
    };

    let visible = $derived.by(() => {
        if (!submenu) return true;

        if (id == $appState.hoverMenuItemGroup) return true;

        return false;
    });

    onMount(() => {
        const _canvas = canvas || (canvas = document.createElement("canvas"));
        const context = _canvas.getContext("2d");
        if (!context) return;
        context.font = 'normal 12px "Segoe UI"';
        let maxWidth = 0;
        const hasAccel = items.some((item) => item.accel);
        const hasSub = items.some((item) => item.type == "submenu");
        hasCheckmark = items.some((item) => item.type == "check" || item.type == "radio");
        items.forEach((item) => {
            const label = item.label ?? "";
            const accel = item.accel ?? "";
            const metrics = context.measureText(label + accel);
            maxWidth = Math.max(maxWidth, metrics.width);
        });
        maxWidth = Math.min(200, maxWidth);
        const padding = 70;
        const accel = hasAccel || hasSub ? 30 : 0;
        width = padding + accel + maxWidth;
    });
</script>

<div class:menu-container={!submenu} class:submenu class:has-checkmark={hasCheckmark} style="min-width: {width}px; visibility:{visible ? 'visible' : 'hidden'}">
    {#each items as item}
        {#if item.type == "submenu"}
            <div
                class="menu-item submenu-container"
                id={item.id}
                class:selected={item.id == $appState.hoverMenuItemGroup}
                data-submenu={item.submenuId}
                {onmouseenter}
                {onmousedown}
                onmouseup={ignore}
                role="button"
                tabindex="-1"
                onkeydown={handleKeyEvent}
            >
                <div class="menu-item-label">{item.label}</div>
                <Menu items={item.items ?? []} id={item.submenuId} submenu={true} />
            </div>
        {:else if item.type == "text"}
            <div
                class="menu-item"
                id={item.id}
                data-accel={item.accel}
                data-value={item.value}
                data-submenu={item.submenuId}
                title={item.id == "History" ? item.label : ""}
                {onmouseenter}
                {onmousedown}
                onmouseup={emit}
                role="button"
                tabindex="-1"
                onkeydown={handleKeyEvent}
            >
                <div class="menu-item-label">{item.label}</div>
            </div>
        {:else if item.type == "check"}
            <div
                class="menu-item checkbox-menu"
                id={item.id}
                data-accel={item.accel}
                data-value={item.value}
                data-submenu={item.submenuId}
                class:checked={item.checked}
                {onmouseenter}
                {onmousedown}
                onmouseup={emit}
                role="button"
                tabindex="-1"
                onkeydown={handleKeyEvent}
            >
                <div class="menu-item-label">{item.label}</div>
            </div>
        {:else if item.type == "radio"}
            <div
                class="menu-item checkbox-menu radio"
                id={item.id}
                data-accel={item.accel}
                data-group={item.id}
                data-value={item.value}
                data-submenu={item.submenuId}
                class:checked={item.checked}
                {onmouseenter}
                {onmousedown}
                onmouseup={emit}
                role="button"
                tabindex="-1"
                onkeydown={handleKeyEvent}
            >
                <div class="menu-item-label">{item.label}</div>
            </div>
        {:else}
            <div class="menu-separator"></div>
        {/if}
    {/each}
</div>

<style>
    .menu-container {
        position: absolute;
        width: fit-content;
        background-color: var(--bar-bg-color);
        display: flex;
        flex-direction: column;
        outline: 1px solid var(--bar-border-color);
        border-radius: 4px;
        font-size: 13px;
        z-index: 9999;
        top: 30px;
        left: 5px;
        box-shadow: 3px 3px 3px 3px var(--menu-shadow);
        padding: 5px 0;
    }

    .menu-item {
        height: 25px;
        padding: 3px 35px 3px 25px;
        color: var(--menu-color);
        cursor: default;
        line-height: 25px;
        position: relative;
    }

    .has-checkmark .menu-item {
        padding: 3px 35px 3px 35px;
    }

    .menu-item::after {
        content: attr(data-accel);
        position: absolute;
        right: 8px;
        height: 100%;
        top: 2px;
        vertical-align: -50%;
        font-size: 12px;
        color: var(--menu-accel-color);
    }

    .menu-item-label {
        max-width: 200px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        pointer-events: none;
        font-size: 12px;
    }

    .selected,
    .menu-item:hover {
        background-color: var(--menu-hover-color);
    }

    .menu-separator {
        width: 100%;
        border-top: 1px solid var(--bar-border-color);
        margin: 5px 0;
    }

    .submenu {
        position: absolute;
        top: -5px;
        left: 100%;
        outline: 1px solid var(--bar-border-color);
        border-radius: 4px;
        visibility: hidden;
        display: flex;
        background-color: var(--bar-bg-color);
        flex-direction: column;
        transition-delay: 0.5s;
        transition-property: visibility;
        box-shadow: 3px 3px 3px 3px var(--menu-shadow);
        padding: 5px 0;
    }

    .submenu-container::before {
        content: "";
        position: absolute;
        right: 11px;
        width: 6px;
        height: 6px;
        top: calc(50% - 4px);
        border-top: 1px solid var(--menu-icon-color);
        border-right: 1px solid var(--menu-icon-color);
        transform: rotate(45deg);
    }

    .checkbox-menu:is(.checked)::before {
        content: "";
        position: absolute;
        display: inline-block;
        transform: rotate(45deg);
        height: 10px;
        width: 5px;
        border-bottom: 1px solid var(--menu-icon-color);
        border-right: 1px solid var(--menu-icon-color);
        left: 16px;
        top: 8px;
    }
</style>
