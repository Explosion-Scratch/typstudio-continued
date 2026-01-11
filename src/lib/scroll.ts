
import type { editor } from "monaco-editor";

/**
 * Preview position interface
 */
export interface PreviewPosition {
    page: number;
    // x, y relative to the page in points (pdf coordinates)
    x: number;
    y: number;
}

// ============================================================================
// EDITOR UTILITIES
// ============================================================================

/**
 * Gets the line numbers for the top, center, and bottom of the visible editor area.
 */
export const getEditor3Lines = (editor: editor.ICodeEditor): [number, number, number] | null => {
    const ranges = editor.getVisibleRanges();
    if (ranges.length === 0) return null;

    // Use the first range (usually the main one)
    const range = ranges[0];
    const top = range.startLineNumber;
    const bottom = range.endLineNumber;
    const center = Math.floor((top + bottom) / 2);

    return [top, center, bottom];
};

/**
 * Gets the byte offsets for the top, center, and bottom visible lines.
 */
export const getEditor3Offsets = (editor: editor.ICodeEditor): [number, number, number] | null => {
    const lines = getEditor3Lines(editor);
    if (!lines) return null;

    const model = editor.getModel();
    if (!model) return null;

    return lines.map(line => {
        const pos = { lineNumber: line, column: 1 };
        return model.getOffsetAt(pos);
    }) as [number, number, number];
};

/**
 * Scrolls the editor so that the target line is at the center of the viewport.
 */
export const scrollEditorToCenterLine = (editor: editor.ICodeEditor, line: number, column: number = 1) => {
    editor.revealLineInCenter(line);
    editor.setPosition({ lineNumber: line, column });
};


// ============================================================================
// PREVIEW UTILITIES
// ============================================================================

/**
 * Calculates the position in the document (page, x, y) for a given absolute Y pixel
 * relative to the pages container top.
 */
const getPositionAtWrapperY = (
    wrapperY: number,
    wrapperX: number,
    pagesContainer: HTMLElement, 
    effectiveScale: number
): PreviewPosition | null => {
    const pageElements = Array.from(pagesContainer.querySelectorAll(".preview-page")) as HTMLElement[];
    
    // We need to find which page contains this Y, or closest.
    // The pages are stacked. wrapperY is 0 at top of first page (usually).
    
    // Pages wrapper might have gaps/padding.
    // pageEl.offsetTop is relative to pagesContainer.
    
    for (const pageEl of pageElements) {
        const pageTop = pageEl.offsetTop;
        const pageHeight = pageEl.clientHeight; 
        const pageBottom = pageTop + pageHeight;
        
        // We add a bit of slack for gaps, or just allow 'closest'.
        // Simple hit test:
        if (wrapperY >= pageTop - 20 && wrapperY <= pageBottom + 20) {
             const pageIndex = parseInt(pageEl.getAttribute("data-page") || "0", 10);
                        
             // Relative Y in document points
             // (WrapperY - PageTop) = PixelYInPage
             // PixelYInPage / Scale = PointY
             
             let relativeY = (wrapperY - pageTop) / effectiveScale;
             // Clamp? Maybe not, allow slight overshot
             relativeY = Math.max(0, relativeY); // Don't go neg
             
             const relativeX = (wrapperX - pageEl.offsetLeft) / effectiveScale;
             
             return {
                 page: pageIndex,
                 x: relativeX,
                 y: relativeY
             };
        }
    }
    
    // Fallback? Return closest?
    return null;
}

/**
 * Gets the document positions for the Top, Center, and Bottom of the current VIEWPORT.
 */
export const getPreview3Positions = (
    container: HTMLElement, 
    pagesContainer: HTMLElement, 
    effectiveScale: number
): [PreviewPosition | null, PreviewPosition | null, PreviewPosition | null] => {
    if (!container || !pagesContainer) return [null, null, null];

    const containerRect = container.getBoundingClientRect();
    const pagesRect = pagesContainer.getBoundingClientRect();
    
    // We want Y positions relative to the PAGES WRAPPER (because that's where pages live)
    // WrapperTop = pagesRect.top;
    
    // Viewport Top in Wrapper Coords:
    // ViewportTopAbs = containerRect.top;
    // Offset = ViewportTopAbs - WrapperTopAbs
    // Wait, "Offset" implies scroll. 
    // Usually: Wrapper is scrolled UP, so WrapperTop is negative relative to ContainerTop?
    // No, Container has `overflow:auto`. PagesWrapper is child.
    // `pagesWrapper.top - container.top` is negative of scrollTop (roughly).
    
    // Correct math:
    // Y_in_Wrapper = Y_in_Viewport - Y_of_Wrapper_Origin_in_Viewport
    
    const wrapperTopInViewport = pagesRect.top;
    const wrapperLeftInViewport = pagesRect.left;
    
    const viewportHeight = container.clientHeight;
    
    // Top of view relative to wrapper:
    // ContainerTop - WrapperTop
    const topY = containerRect.top - wrapperTopInViewport;
    const centerY = topY + (viewportHeight / 2);
    const bottomY = topY + viewportHeight;
    
    // X center
    const centerX = (containerRect.left + containerRect.width / 2) - wrapperLeftInViewport;
    
    return [
       getPositionAtWrapperY(topY, centerX, pagesContainer, effectiveScale),
       getPositionAtWrapperY(centerY, centerX, pagesContainer, effectiveScale),
       getPositionAtWrapperY(bottomY, centerX, pagesContainer, effectiveScale)
    ];
};

/**
 * Calculates the scroll position required to put the given preview position 
 * in the CENTER of the viewport.
 */
export const calculatePreviewScrollCenter = (
    container: HTMLElement,
    pageElement: HTMLElement,
    previewPos: PreviewPosition,
    effectiveScale: number
): { top: number, left: number } | null => {
   if (!container || !pageElement) return null;

   const pageRect = pageElement.getBoundingClientRect();
   const containerRect = container.getBoundingClientRect();
   
   // Current absolute positions
   const currentScrollTop = container.scrollTop;
   const currentScrollLeft = container.scrollLeft;
   
   // Pixel position relative to page top-left
   const pixelY = previewPos.y * effectiveScale;
   const pixelX = previewPos.x * effectiveScale;
   
   // Top of the page relative to the scrolled content area (0,0)
   // pageRect.top is viewport coord.
   // absolutePageTop = currentScrollTop + (pageRect.top - containerRect.top)
   const absolutePageTop = currentScrollTop + (pageRect.top - containerRect.top);
   const absolutePageLeft = currentScrollLeft + (pageRect.left - containerRect.left);
   
   // Absolute target pixel
   const targetAbsY = absolutePageTop + pixelY;
   const targetAbsX = absolutePageLeft + pixelX;
   
   // We want targetAbsY to be at Center of viewport.
   // Viewport Center in Absolute Coords = NewScrollTop + ViewportHeight/2
   // NewScrollTop = TargetAbsY - ViewportHeight/2
   
   const targetTop = targetAbsY - (container.clientHeight / 2);
   const targetLeft = targetAbsX - (container.clientWidth / 2);
   
   return { top: targetTop, left: targetLeft };
}
// ... (existing exports)

import { jump, jumpFromCursor } from "./ipc";
import type { TypstDocumentPosition } from "./ipc";

/**
 * Calculates the robust sync target from Editor -> Preview.
 * Samples Top, Center, and Bottom lines, and returns the Center position as the reference.
 */
/**
 * Gets the line number for the center visible line.
 */
export const getEditorCenterLine = (editor: editor.ICodeEditor): number | null => {
    const ranges = editor.getVisibleRanges();
    if (ranges.length === 0) return null;
    const range = ranges[0];
    return Math.floor((range.startLineNumber + range.endLineNumber) / 2);
};

/**
 * Gets the byte offset for the center visible line.
 */
export const getEditorCenterOffset = (editor: editor.ICodeEditor): number | null => {
    const line = getEditorCenterLine(editor);
    if (!line) return null;
    const model = editor.getModel();
    if (!model) return null;
    return model.getOffsetAt({ lineNumber: line, column: 1 });
};

/**
 * Calculates the sync target from Editor -> Preview.
 * Uses ONLY the Center line to determine the sync target.
 */
export const getEditorToPreviewTarget = async (editor: editor.ICodeEditor): Promise<TypstDocumentPosition | null> => {
    const offset = getEditorCenterOffset(editor);
    if (offset === null) return null;

    const model = editor.getModel();
    if (!model) return null;
    
    const content = model.getValue();
    const uri = model.uri.path;
    const encoder = new TextEncoder();
    
    // Calculate byte offset
    const byteOffset = encoder.encode(content.substring(0, offset)).length;

    try {
        return await jumpFromCursor(uri, content, byteOffset);
    } catch (e) {
        console.error("Failed to calculate editor sync target", e);
        return null;
    }
}

/**
 * Calculates the robust sync target from Preview -> Editor.
 * Samples Top, Center, and Bottom of viewport, averages the resulting lines.
 */
export const getPreviewToEditorTargetLine = async (
    container: HTMLElement, 
    pagesContainer: HTMLElement, 
    effectiveScale: number
): Promise<number | null> => {
    const positions = getPreview3Positions(container, pagesContainer, effectiveScale);
    
    try {
        const promises = positions.map(async pos => {
            if (!pos) return null;
            try {
                return await jump(pos.page, pos.x, pos.y);
            } catch {
                return null;
            }
        });
        
        const results = await Promise.all(promises);
        
        const lines: number[] = [];
        for (const r of results) {
            if (r && r.start && typeof r.start[0] === 'number') {
                lines.push(r.start[0]);
            }
        }
        
        if (lines.length === 0) return null;
        
        const sum = lines.reduce((a, b) => a + b, 0);
        return Math.round(sum / lines.length);
    } catch (e) {
        console.error("Failed to calculate preview sync target", e);
        return null;
    }
}
