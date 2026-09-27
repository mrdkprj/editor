<script lang="ts">
    import { scale } from "svelte/transition";
    import type { Snippet } from "svelte";

    export type DialogColors = {
        background: string;
        color: string;
        shadow: string;
        outline: string;
        closeHoverBackground: string;
        closeHoverColor: string;
        separator: string;
    };

    let {
        title = "",
        header,
        content,
        action,
        focusOnMount = false,
        overlayOffet = 0,
        width,
        height,
        minWidth,
        minHeight,
        colors,
        close,
        onkeydown,
    }: {
        title?: string;
        header?: Snippet;
        content: Snippet;
        action?: Snippet;
        focusOnMount?: boolean;
        overlayOffet?: number;
        width?: number;
        height?: number;
        minWidth?: number;
        minHeight?: number;
        colors: DialogColors;
        close: () => void;
        onkeydown?: (e: KeyboardEvent) => void;
    } = $props();

    const position = $state({ x: 0, y: 0 });
    const dragState = {
        dragging: false,
        bounds: { x: 0, y: 0 },
        start: { x: 0, y: 0 },
        offset: { x: 0, y: 0 },
    };
    let dialogWidth = $state(0);
    let dialogHeight = $state(0);

    const _onkeydown = (e: KeyboardEvent) => {
        if (e.key == "Escape") {
            close();
        }

        if (onkeydown) {
            onkeydown(e);
        }
    };

    const setKeyboardFocus = (node: HTMLDivElement) => {
        if (focusOnMount) {
            node.focus();
        }
    };

    const onmousedown = (e: MouseEvent) => {
        const w = width ?? dialogWidth;
        const h = height ?? dialogHeight;
        const x = (window.innerWidth - w) / 2;
        const y = (window.innerHeight - h - overlayOffet) / 2;
        dragState.dragging = true;
        dragState.bounds = { x, y };
        dragState.offset = { x: position.x, y: position.y };
        dragState.start = { x: e.clientX, y: e.clientY };
    };

    const onmouseup = () => {
        dragState.dragging = false;
    };

    const onmousemove = (e: MouseEvent) => {
        if (!dragState.dragging) return;
        const moveX = dragState.offset.x + (e.clientX - dragState.start.x);
        const moveY = dragState.offset.y + (e.clientY - dragState.start.y);
        position.x = Math.abs(moveX) >= dragState.bounds.x || moveX == 0 ? position.x : moveX;
        position.y = Math.abs(moveY) >= dragState.bounds.y || moveY == 0 ? position.y : moveY;
    };

    const onWindowResize = () => {
        position.x = 0;
        position.y = 0;
    };

    const handleKeyEvent = () => {};
</script>

<svelte:window onresize={onWindowResize} />
<svelte:document {onmousemove} {onmouseup} />
<div
    class="dialog-overlay"
    style={`height:calc(100% - ${overlayOffet}px); top:${overlayOffet}px;`}
    style:--background={colors.background}
    style:--color={colors.color}
    style:--shadow={colors.shadow}
    style:--outline={colors.outline}
    style:--closeHoverBackground={colors.closeHoverBackground}
    style:--closeHoverColor={colors.closeHoverColor}
    style:--seperatorColor={colors.separator}
    onkeydown={_onkeydown}
    role="button"
    tabindex="-1"
    transition:scale={{ delay: 0, duration: 100 }}
    use:setKeyboardFocus
>
    <div
        class="dialog-container"
        style:width={width ? `${width}px;` : "fit-content;"}
        style:height={height ? `${height}px;` : "fit-content;"}
        style={`min-width:${minWidth}px; min-height:${minHeight}px; transform: translate(${position.x}px, ${position.y}px);`}
        bind:clientWidth={dialogWidth}
        bind:clientHeight={dialogHeight}
    >
        <div class="dialog-header" {onmousedown} onkeydown={handleKeyEvent} role="button" tabindex="-1">
            {#if header}
                {@render header()}
            {/if}
            <div class="dialog-close" onclick={close} onkeydown={handleKeyEvent} role="button" tabindex="-1">&times;</div>
        </div>
        <div class="dialog">
            {#if title}
                <div class="dialog-title-block">{title}</div>
            {/if}
            {@render content()}
            {#if action}
                <div class="dialog-separator"></div>
                <div class="dialog-action">
                    {@render action()}
                </div>
            {/if}
        </div>
    </div>
</div>

<style>
    .dialog-overlay {
        background-color: transparent;
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        position: absolute;
        z-index: 9999;
        left: 0px;
        outline: none;
    }

    .dialog-container {
        background-color: var(--background);
        color: var(--color);
        display: flex;
        flex-direction: column;
        box-shadow: 7px 5px 5px var(--shadow);
        outline: 1px solid var(--outline);
        border-radius: 8px;
    }

    .dialog-header {
        width: 100%;
        height: 30px;
        display: flex;
        justify-content: flex-end;
        align-items: center;
    }

    .dialog {
        width: calc(100% - 40px);
        height: calc(100% - 30px);
        overflow: auto;
        padding: 0 20px 20px 20px;
    }

    :global(.dialog-item-block) {
        font-size: 14px;
        margin-bottom: 10px;
    }

    :global(.dialog-separator) {
        height: 1px;
        border-top: 1px solid var(--seperatorColor);
        margin: 20px 0 10px 0;
    }

    :global(.dialog-item) {
        display: flex;
        align-items: center;
        margin-bottom: 5px;
    }

    :global(.dialog-item label) {
        user-select: none;
    }

    :global(.dialog-title-block) {
        font-size: 16px;
        font-weight: bold;
        margin: 0 0 10px 0;
    }

    .dialog-close {
        align-items: center;
        display: flex;
        justify-content: center;
        width: 45px;
        height: 100%;
        text-align: center;
        user-select: none;
        -webkit-user-select: none;
        position: relative;
        -webkit-app-region: no-drag;
        line-height: 35px;
        border-top-right-radius: 4px;
    }

    .dialog-close:hover {
        background-color: var(--closeHoverBackground);
        color: var(--closeHoverColor);
    }

    .dialog-action {
        display: flex;
        align-items: center;
        justify-content: flex-end;
    }

    :global(.dialog-action button:not(:last-child)) {
        margin-right: 10px;
    }

    :global(.dialog-btn-lg) {
        padding: 8px;
    }

    :global(.dialog-btn-lgw) {
        padding: 8px 20px;
    }

    :global(.dialog-btn-md) {
        padding: 5px;
    }

    :global(.dialog-btn-md:disabled) {
        background-color: #ccc;
        pointer-events: none;
    }

    :global(.dialog-action button:first-child) {
        margin-right: 10px;
    }
</style>
