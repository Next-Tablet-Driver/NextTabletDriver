<script lang="ts">
  import { X } from "lucide-svelte";
  import type { Snippet } from "svelte";

  interface Props {
      title: string;
      onClose: () => void;
      children: Snippet;
  }

  const { title, onClose, children }: Props = $props();

  let isDragging = false;
  let startX = 0;
  let startY = 0;
  let initialX = 0;
  let initialY = 0;

  // Center by default, but allow dragging
  let transformX = $state(100);
  let transformY = $state(100);

  function onMouseDown(e: MouseEvent): void {
      isDragging = true;
      startX = e.clientX;
      startY = e.clientY;
      initialX = transformX;
      initialY = transformY;
      
      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
  }

  function onMouseMove(e: MouseEvent): void {
      if (!isDragging) return;
      const dx = e.clientX - startX;
      const dy = e.clientY - startY;
      transformX = initialX + dx;
      transformY = initialY + dy;
  }

  function onMouseUp(): void {
      isDragging = false;
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
  }
</script>

<div 
  style:transform="translate({transformX}px, {transformY}px)" 
  class="draggable-panel"
>
  <div class="panel-header" onmousedown={onMouseDown} role="presentation">
      <span class="title">{title}</span>
      <button class="close-btn" onclick={onClose}>
          <X size={16} />
      </button>
  </div>
  <div class="panel-content">
      {@render children()}
  </div>
</div>

<style>
  .draggable-panel {
      position: absolute;
      top: 0;
      left: 0;
      width: 500px;
      height: 400px;
      background: var(--bg-app);
      border: var(--border-width) solid var(--border);
      border-radius: var(--radius-lg);
      display: flex;
      flex-direction: column;
      box-shadow: var(--shadow-floating);
      z-index: var(--z-floating);
      overflow: hidden;
  }

  .panel-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: var(--space-2) var(--space-3);
      background: var(--bg-panel);
      border-bottom: var(--border-width) solid var(--border);
      cursor: grab;
      user-select: none;
  }

  .panel-header:active {
      cursor: grabbing;
  }

  .title {
      font-weight: var(--font-weight-bold);
      font-size: var(--font-md);
      color: var(--text-main);
  }

  .close-btn {
      background: transparent;
      border: none;
      color: var(--text-muted);
      cursor: pointer;
      padding: var(--space-0-5);
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: var(--radius-md);
  }

  .close-btn:hover {
      background: var(--overlay-10);
      color: var(--text-main);
  }

  .panel-content {
      flex: 1;
      padding: var(--space-4);
      overflow-y: auto;
      color: var(--text-main);
  }
</style>
