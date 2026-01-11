
import type { editor } from "monaco-editor";

export interface PreviewPosition {
    page: number;
    x: number;
    y: number;
}

export const getEditor3Lines = (editor: editor.ICodeEditor): [number, number, number] | null => {
    const ranges = editor.getVisibleRanges();
    if (ranges.length === 0) return null;

    const range = ranges[0];
    const top = range.startLineNumber;
    const bottom = range.endLineNumber;
    const center = Math.floor((top + bottom) / 2);

    return [top, center, bottom];
};

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

export const scrollEditorToCenterLine = (editor: editor.ICodeEditor, line: number, column: number = 1) => {
    editor.revealLineInCenter(line);
    editor.setPosition({ lineNumber: line, column });
};

const getPositionAtWrapperY = (
    wrapperY: number,
    wrapperX: number,
    pagesContainer: HTMLElement, 
    effectiveScale: number
): PreviewPosition | null => {
    const pageElements = Array.from(pagesContainer.querySelectorAll(".preview-page")) as HTMLElement[];
    
    for (const pageEl of pageElements) {
        const pageTop = pageEl.offsetTop;
        const pageHeight = pageEl.clientHeight; 
        const pageBottom = pageTop + pageHeight;
        
        if (wrapperY >= pageTop - 20 && wrapperY <= pageBottom + 20) {
             const pageIndex = parseInt(pageEl.getAttribute("data-page") || "0", 10);
                        
             let relativeY = (wrapperY - pageTop) / effectiveScale;
             relativeY = Math.max(0, relativeY);
             
             const relativeX = (wrapperX - pageEl.offsetLeft) / effectiveScale;
             
             return {
                 page: pageIndex,
                 x: relativeX,
                 y: relativeY
             };
        }
    }
    
    return null;
}

export const getPreview3Positions = (
    container: HTMLElement, 
    pagesContainer: HTMLElement, 
    effectiveScale: number
): [PreviewPosition | null, PreviewPosition | null, PreviewPosition | null] => {
    if (!container || !pagesContainer) return [null, null, null];

    const containerRect = container.getBoundingClientRect();
    const pagesRect = pagesContainer.getBoundingClientRect();
    
    const wrapperTopInViewport = pagesRect.top;
    const wrapperLeftInViewport = pagesRect.left;
    
    const viewportHeight = container.clientHeight;
    
    const topY = containerRect.top - wrapperTopInViewport;
    const centerY = topY + (viewportHeight / 2);
    const bottomY = topY + viewportHeight;
    
    const centerX = (containerRect.left + containerRect.width / 2) - wrapperLeftInViewport;
    
    return [
       getPositionAtWrapperY(topY, centerX, pagesContainer, effectiveScale),
       getPositionAtWrapperY(centerY, centerX, pagesContainer, effectiveScale),
       getPositionAtWrapperY(bottomY, centerX, pagesContainer, effectiveScale)
    ];
};

export const calculatePreviewScrollCenter = (
    container: HTMLElement,
    pageElement: HTMLElement,
    previewPos: PreviewPosition,
    effectiveScale: number
): { top: number, left: number } | null => {
   if (!container || !pageElement) return null;

   const pageRect = pageElement.getBoundingClientRect();
   const containerRect = container.getBoundingClientRect();
   
   const currentScrollTop = container.scrollTop;
   const currentScrollLeft = container.scrollLeft;
   
   const pixelY = previewPos.y * effectiveScale;
   const pixelX = previewPos.x * effectiveScale;
   
   const absolutePageTop = currentScrollTop + (pageRect.top - containerRect.top);
   const absolutePageLeft = currentScrollLeft + (pageRect.left - containerRect.left);
   
   const targetAbsY = absolutePageTop + pixelY;
   const targetAbsX = absolutePageLeft + pixelX;
   
   const targetTop = targetAbsY - (container.clientHeight / 2);
   const targetLeft = targetAbsX - (container.clientWidth / 2);
   
   return { top: targetTop, left: targetLeft };
}

import { jump, jumpFromCursor } from "./ipc";
import type { TypstDocumentPosition } from "./ipc";

export const getEditorCenterLine = (editor: editor.ICodeEditor): number | null => {
    const ranges = editor.getVisibleRanges();
    if (ranges.length === 0) return null;
    const range = ranges[0];
    return Math.floor((range.startLineNumber + range.endLineNumber) / 2);
};

export const getEditorCenterOffset = (editor: editor.ICodeEditor): number | null => {
    const line = getEditorCenterLine(editor);
    if (!line) return null;
    const model = editor.getModel();
    if (!model) return null;
    return model.getOffsetAt({ lineNumber: line, column: 1 });
};

export const getEditorToPreviewTarget = async (editor: editor.ICodeEditor): Promise<TypstDocumentPosition | null> => {
    const offset = getEditorCenterOffset(editor);
    if (offset === null) return null;

    const model = editor.getModel();
    if (!model) return null;
    
    const content = model.getValue();
    const uri = model.uri.path;
    const encoder = new TextEncoder();
    
    const byteOffset = encoder.encode(content.substring(0, offset)).length;

    try {
        return await jumpFromCursor(uri, content, byteOffset);
    } catch (e) {
        console.error("Failed to calculate editor sync target", e);
        return null;
    }
}

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
