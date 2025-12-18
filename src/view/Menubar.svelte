<script lang="ts">
    import { appState, dispatch, textState, temporal, settings } from "./appStateReducer.svelte";
    import { EDIT_MENU_ITEMS, handleKeyEvent } from "../constants";
    import util from "../util";
    import Menu from "./Menu.svelte";

    let fileMenuItems = $derived(util.getFileMenubarItems(settings.history, textState.encoding));
    let viewMenuItems = $derived(util.getViewMenubarItems(settings.theme, temporal[textState.textType]));

    const onMenuBarItemMousedown = (e: MouseEvent) => {
        if (!e.target || !(e.target instanceof HTMLElement)) return;

        e.preventDefault();
        if ($appState.visibleMenubarItem == e.target.id) {
            dispatch({ type: "visibleMenubarItem", value: "" });
            return;
        }
        dispatch({ type: "openingMenu", value: true });
        dispatch({ type: "visibleMenubarItem", value: e.target.id });
    };

    const onMenuBarItemMouseenter = (id: string) => {
        if ($appState.visibleMenubarItem) {
            dispatch({ type: "visibleMenubarItem", value: id });
        }
    };
</script>

<div class="menu-bar no-print">
    <div class="menu-bar-item">
        <div
            id="file"
            class="menu-bar-item-label"
            class:menu-bar-item-label-selected={$appState.visibleMenubarItem == "file"}
            onmouseenter={() => onMenuBarItemMouseenter("file")}
            onmousedown={onMenuBarItemMousedown}
            role="button"
            tabindex="-1"
            onkeydown={handleKeyEvent}
        >
            File
        </div>
        {#if $appState.visibleMenubarItem == "file"}
            <Menu items={fileMenuItems} submenu={false} />
        {/if}
    </div>
    <div class="menu-bar-item">
        <div
            id="edit"
            class="menu-bar-item-label"
            class:menu-bar-item-label-selected={$appState.visibleMenubarItem == "edit"}
            onmouseenter={() => onMenuBarItemMouseenter("edit")}
            onmousedown={onMenuBarItemMousedown}
            role="button"
            tabindex="-1"
            onkeydown={handleKeyEvent}
        >
            Edit
        </div>
        {#if $appState.visibleMenubarItem == "edit"}
            <Menu items={EDIT_MENU_ITEMS} submenu={false} />
        {/if}
    </div>
    <div class="menu-bar-item">
        <div
            id="view"
            class="menu-bar-item-label"
            class:menu-bar-item-label-selected={$appState.visibleMenubarItem == "view"}
            onmouseenter={() => onMenuBarItemMouseenter("view")}
            onmousedown={onMenuBarItemMousedown}
            role="button"
            tabindex="-1"
            onkeydown={handleKeyEvent}
        >
            View
        </div>
        {#if $appState.visibleMenubarItem == "view"}
            <Menu items={viewMenuItems} submenu={false} />
        {/if}
    </div>
</div>

<style>
    .menu-bar {
        display: flex;
        flex-direction: row;
        align-items: center;
        justify-content: flex-start;
        height: 35px;
    }

    .menu-bar-item {
        height: 100%;
        padding: 0px 4px;
        font-size: 14px;
        position: relative;
        user-select: none;
        -webkit-user-select: none;
        -webkit-app-region: no-drag;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .menu-bar-item-label {
        padding: 4px 8px;
        user-select: none;
        -webkit-user-select: none;
        -webkit-app-region: no-drag;
    }

    .menu-bar-item-label:hover,
    .menu-bar-item-label-selected {
        background-color: var(--menu-hover-color);
        border-radius: 4px;
    }

    .menu-bar-item:focus-visible {
        outline: none;
    }
</style>
