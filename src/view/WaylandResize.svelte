<script lang="ts">
    import { getCurrentWebviewWindow, WebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { settings } from "./appStateReducer.svelte";

    type ResizeDirection = "East" | "North" | "NorthEast" | "NorthWest" | "South" | "SouthEast" | "SouthWest" | "West";
    const dragResize = async (e: MouseEvent) => {
        if (!e.target || !(e.target instanceof HTMLElement)) return;
        const direction = e.target.getAttribute("data-direction");
        if (direction) {
            if (settings.tabMode) {
                const main = await WebviewWindow.getByLabel("Main");
                main?.startResizeDragging(direction as ResizeDirection);
            } else {
                getCurrentWebviewWindow().startResizeDragging(direction as ResizeDirection);
            }
        }
    };
</script>

<div class="resize-handle edge top" data-direction="North" onmousedown={dragResize} role="button" tabindex="-1"></div>
<div class="resize-handle corner top-right" data-direction="NorthEast" onmousedown={dragResize} role="button" tabindex="-1"></div>
<div class="resize-handle edge right" data-direction="East" onmousedown={dragResize} role="button" tabindex="-1"></div>
<div class="resize-handle corner bottom-right" data-direction="SouthEast" onmousedown={dragResize} role="button" tabindex="-1"></div>
<div class="resize-handle edge bottom" data-direction="South" onmousedown={dragResize} role="button" tabindex="-1"></div>
<div class="resize-handle corner bottom-left" data-direction="SouthWest" onmousedown={dragResize} role="button" tabindex="-1"></div>
<div class="resize-handle edge left" data-direction="West" onmousedown={dragResize} role="button" tabindex="-1"></div>
<div class="resize-handle corner top-left" data-direction="NorthWest" onmousedown={dragResize} role="button" tabindex="-1"></div>

<style>
    /* Base style for all drag hitboxes */
    .resize-handle {
        position: absolute;
        user-select: none;
    }

    /* ==========================================================================
   4 EDGES (Thickness: 6px)
   ========================================================================== */
    .resize-handle.edge {
        z-index: 9998; /* Lower than corners */
    }
    .resize-handle.edge.top {
        top: 0;
        left: 6px;
        right: 6px;
        height: 6px;
        cursor: ns-resize;
    }
    .resize-handle.edge.bottom {
        bottom: 0;
        left: 6px;
        right: 6px;
        height: 6px;
        cursor: ns-resize;
    }
    .resize-handle.edge.left {
        left: 0;
        top: 6px;
        bottom: 6px;
        width: 6px;
        cursor: ew-resize;
    }
    .resize-handle.edge.right {
        right: 0;
        top: 6px;
        bottom: 6px;
        width: 6px;
        cursor: ew-resize;
    }

    /* ==========================================================================
   4 CORNERS (Size: 10px x 10px to overlap edges cleanly)
   ========================================================================== */
    .resize-handle.corner {
        width: 10px;
        height: 10px;
        z-index: 9999; /* Higher priority to overlay over edge joints */
    }
    .resize-handle.corner.top-left {
        top: 0;
        left: 0;
        cursor: nwse-resize;
    }
    .resize-handle.corner.top-right {
        top: 0;
        right: 0;
        cursor: nesw-resize;
    }
    .resize-handle.corner.bottom-left {
        bottom: 0;
        left: 0;
        cursor: nesw-resize;
    }
    .resize-handle.corner.bottom-right {
        bottom: 0;
        right: 0;
        cursor: nwse-resize;
    }
</style>
