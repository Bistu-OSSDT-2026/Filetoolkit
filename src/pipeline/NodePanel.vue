<script setup lang="ts">
import {
  Picture,
  Document,
  EditPen,
  Search,
  VideoCamera,
  FolderOpened,
} from "@element-plus/icons-vue";
import type { Component } from "vue";
import { groupedNodeTypes } from "../composables/usePipeline";

const emit = defineEmits<{
  (e: "drag-start", nodeTypeId: string, event: DragEvent): void;
  /** 点击节点类型时触发（Tauri WebView2 拖放不可用时的备选方案） */
  (e: "add-node", nodeTypeId: string): void;
}>();

const groups = groupedNodeTypes();

/** 节点图标映射 */
const iconMap: Record<string, Component> = {
  Picture,
  Document,
  EditPen,
  Search,
  VideoCamera,
  FolderOpened,
};

function onDragStart(nodeTypeId: string, event: DragEvent) {
  if (event.dataTransfer) {
    event.dataTransfer.setData("application/vueflow-node-type", nodeTypeId);
    event.dataTransfer.setData("text/plain", nodeTypeId);
    event.dataTransfer.effectAllowed = "move";
  }
  // 标记拖拽进行中，阻止 click 误触发 add-node
  isDragging = true;
  emit("drag-start", nodeTypeId, event);
}

/** 点击添加节点（Tauri WebView2 拖放不支持时的备选方案） */
function onClickAdd(nodeTypeId: string) {
  // 如果正在拖拽（dragstart 刚触发），忽略 click
  if (isDragging) {
    isDragging = false;
    return;
  }
  emit("add-node", nodeTypeId);
}

/** 拖拽进行中标记 — click 和 dragstart 都会在 mousedown 时触发，用此标记区分 */
let isDragging = false;
</script>

<template>
  <div class="node-panel">
    <h3 class="panel-title">节点类型</h3>
    <p class="panel-hint">拖拽节点到画布</p>

    <div v-for="[category, nodes] in groups" :key="category" class="category-group">
      <h4 class="category-title">{{ category }}</h4>
      <div
        v-for="nt in nodes"
        :key="nt.id"
        class="node-item"
        draggable="true"
        title="拖拽到画布或点击添加"
        @dragstart="onDragStart(nt.id, $event)"
        @click="onClickAdd(nt.id)"
      >
        <el-icon :size="18">
          <component :is="iconMap[nt.icon ?? ''] ?? Document" />
        </el-icon>
        <div class="node-item-info">
          <span class="node-item-name">{{ nt.name }}</span>
          <span class="node-item-desc">{{ nt.description }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.node-panel {
  padding: 12px;
  height: 100%;
  overflow-y: auto;
  user-select: none;
}

.panel-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}

.panel-hint {
  margin: 4px 0 16px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.category-group {
  margin-bottom: 16px;
}

.category-title {
  margin: 0 0 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--el-color-primary);
}

.node-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  margin-bottom: 4px;
  border: 1px solid var(--el-border-color-light);
  border-radius: 6px;
  cursor: grab;
  transition:
    background-color 0.15s,
    border-color 0.15s;
}

.node-item:hover {
  background-color: var(--el-color-primary-light-9);
  border-color: var(--el-color-primary-light-5);
}

.node-item:active {
  cursor: grabbing;
}

.node-item-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.node-item-name {
  font-size: 13px;
  font-weight: 500;
}

.node-item-desc {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
