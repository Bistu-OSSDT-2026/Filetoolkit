<script setup lang="ts">
import { ref, onMounted, onUnmounted, onBeforeUnmount } from "vue";
import { VueFlow, useVueFlow } from "@vue-flow/core";
import { Background } from "@vue-flow/background";
import { Controls } from "@vue-flow/controls";
import { MiniMap } from "@vue-flow/minimap";
import "@vue-flow/core/dist/style.css";
import "@vue-flow/core/dist/theme-default.css";
import "@vue-flow/controls/dist/style.css";
import "@vue-flow/minimap/dist/style.css";
import { ElMessage, ElMessageBox } from "element-plus";
import { Delete, Upload, Download, Check, RefreshLeft, Files } from "@element-plus/icons-vue";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import NodePanel from "./NodePanel.vue";
import NodeParamForm from "./NodeParamForm.vue";
import TemplateManager from "./TemplateManager.vue";
import { PipelineState } from "../composables/usePipeline";
import type { Pipeline } from "../pipeline/types";

const { onConnect, onNodeClick, onPaneClick, screenToFlowCoordinate } = useVueFlow();

// PipelineState 管理自己的 flowNodes/flowEdges，不与 useVueFlow() 共享 ref
const pipeline = new PipelineState();
const showValidationErrors = ref(false);
const showTemplateDialog = ref(false);

/** .flow-canvas 的 DOM 引用，用于注册原生事件监听 */
const flowCanvasRef = ref<HTMLElement | null>(null);

function onDragEnter(event: DragEvent) {
  event.preventDefault();
}

function onDragOver(event: DragEvent) {
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
}

function onDrop(event: DragEvent) {
  event.preventDefault();
  const dt = event.dataTransfer;
  if (!dt) return;
  const nodeTypeId =
    dt.getData("application/vueflow-node-type") ||
    dt.getData("text/plain");
  if (!nodeTypeId) return;
  pipeline.addNode(nodeTypeId, screenToFlowCoordinate({ x: event.clientX, y: event.clientY }));
}

/** 备选：点击添加节点（Tauri WebView2 拖放不可用时的 fallback） */
function handleClickAddNode(nodeTypeId: string) {
  pipeline.addNode(nodeTypeId, { x: 250, y: 150 });
}

// ========== 原生事件绑定（CAPTURE 阶段）：在 VueFlow 内部消费事件之前拦截 ==========
// VueFlow 的 .vue-flow__pane 在 bubble 阶段会调用 stopPropagation()，
// 导致绑定在外层的冒泡监听器收不到事件。
// 解决方法：使用 capture 阶段（第三个参数 true），从外层向里层传播时优先触发。

function bindNativeDragEvents() {
  const el = flowCanvasRef.value;
  if (!el) return;
  // true = capture phase，优先于 VueFlow 内部 .vue-flow__pane 的 bubble handler
  el.addEventListener("dragenter", onDragEnter, true);
  el.addEventListener("dragover", onDragOver, true);
  el.addEventListener("drop", onDrop, true);
}

function unbindNativeDragEvents() {
  const el = flowCanvasRef.value;
  if (!el) return;
  el.removeEventListener("dragenter", onDragEnter, true);
  el.removeEventListener("dragover", onDragOver, true);
  el.removeEventListener("drop", onDrop, true);
}

onConnect((connection) => pipeline.addEdge(connection));

onNodeClick(({ node }) => {
  pipeline.selectedNodeId.value = node.id;
});
onPaneClick(() => {
  pipeline.selectedNodeId.value = null;
});

function handleDelete() {
  if (!pipeline.selectedNodeId.value) {
    ElMessage.warning("请先选中一个节点");
    return;
  }
  pipeline.removeNode(pipeline.selectedNodeId.value);
}
function handleValidate() {
  const errors = pipeline.validate();
  if (errors.length === 0) {
    showValidationErrors.value = false;
    ElMessage.success("校验通过，流水线结构正确");
  } else {
    showValidationErrors.value = true;
    ElMessage.error(`校验失败: ${errors.length} 个问题`);
  }
}
function handleClear() {
  ElMessageBox.confirm("确定清空画布？此操作不可撤销。", "确认", {
    confirmButtonText: "清空",
    cancelButtonText: "取消",
    type: "warning",
  })
    .then(() => pipeline.clearCanvas())
    .catch(() => {});
}
async function handleExport() {
  const json = pipeline.toPipelineJSON();
  const content = JSON.stringify(json, null, 2);
  const defaultName = `${pipeline.pipelineName.value || "pipeline"}.json`;

  // 检测是否在 Tauri 环境（有原生对话框可用）
  if (window.__TAURI_INTERNALS__) {
    try {
      const filePath = await save({
        defaultPath: defaultName,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!filePath) return; // 用户取消
      await writeTextFile(filePath, content);
      ElMessage.success("流水线已导出");
    } catch (e) {
      ElMessage.error("导出失败: " + String(e));
    }
    return;
  }

  // 浏览器环境：blob 下载
  const blob = new Blob([content], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = defaultName;
  a.click();
  URL.revokeObjectURL(url);
  ElMessage.success("流水线已导出");
}
function handleImport() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".json";
  input.onchange = async () => {
    const file = input.files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      const json = JSON.parse(text);
      if (!json.nodes || !json.edges) {
        ElMessage.error("无效的流水线文件: 缺少 nodes 或 edges");
        return;
      }
      pipeline.fromPipelineJSON(json);
      ElMessage.success(`已加载流水线: ${json.name || "未命名"}`);
    } catch {
      ElMessage.error("无法解析流水线文件");
    }
  };
  input.click();
}
function handleLoadTemplate(p: Pipeline) {
  pipeline.fromPipelineJSON(p);
  showTemplateDialog.value = false;
}
function onKeydown(e: KeyboardEvent) {
  if (
    (e.key === "Delete" || e.key === "Backspace") &&
    pipeline.selectedNodeId.value &&
    document.activeElement === document.body
  ) {
    pipeline.removeNode(pipeline.selectedNodeId.value);
  }
}
onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  bindNativeDragEvents();
});
onBeforeUnmount(() => {
  // 清空画布数据，强制 Vue Flow 释放内部状态
  pipeline.flowNodes.value = [];
  pipeline.flowEdges.value = [];
});
onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  unbindNativeDragEvents();
});
</script>

<template>
  <div class="pipeline-editor">
    <aside class="editor-left"><NodePanel @add-node="handleClickAddNode" /></aside>
    <main class="editor-center">
      <div class="canvas-toolbar">
        <div class="toolbar-left">
          <el-input
            v-model="pipeline.pipelineName.value"
            placeholder="流水线名称"
            size="small"
            style="width: 200px"
          />
        </div>
        <div class="toolbar-right">
          <el-button size="small" @click="handleImport">
            <el-icon><Upload /></el-icon>导入
          </el-button>
          <el-button size="small" @click="handleExport">
            <el-icon><Download /></el-icon>导出
          </el-button>
          <el-button size="small" type="danger" text @click="handleDelete">
            <el-icon><Delete /></el-icon>删除节点
          </el-button>
          <el-button size="small" type="primary" @click="handleValidate">
            <el-icon><Check /></el-icon>校验
          </el-button>
          <el-button size="small" @click="showTemplateDialog = true">
            <el-icon><Files /></el-icon>模板
          </el-button>
          <el-button size="small" text @click="handleClear">
            <el-icon><RefreshLeft /></el-icon>清空
          </el-button>
        </div>
      </div>
      <el-alert
        v-if="showValidationErrors && pipeline.validationErrors.value.length > 0"
        type="error"
        :closable="true"
        style="margin: 8px 16px 0"
        @close="showValidationErrors = false"
      >
        <template #title>
          <ul style="margin: 0; padding-left: 16px">
            <li v-for="err in pipeline.validationErrors.value" :key="err">{{ err }}</li>
          </ul>
        </template>
      </el-alert>
      <!-- 拖放事件仅通过原生 addEventListener (capture 阶段) 处理 -->
      <div ref="flowCanvasRef" class="flow-canvas">
        <VueFlow
          :nodes="pipeline.flowNodes.value"
          :edges="pipeline.flowEdges.value"
          @update:nodes="pipeline.acceptFlowNodes($event)"
          @update:edges="pipeline.acceptFlowEdges($event)"
          :default-viewport="{ x: 0, y: 0, zoom: 1 }"
          :min-zoom="0.2"
          :max-zoom="4"
          :snap-to-grid="true"
          :snap-grid="[20, 20]"
          fit-view-on-init
        >
          <Background :gap="20" />
          <Controls position="bottom-right" />
          <MiniMap position="bottom-left" />
        </VueFlow>
      </div>
    </main>
    <aside class="editor-right">
      <NodeParamForm
        :node-type="pipeline.getSelectedNodeType()"
        :params="pipeline.getSelectedNodeParams()"
        @update:param="
          (key: string, value: unknown) =>
            pipeline.updateNodeParam(pipeline.selectedNodeId.value!, key, value)
        "
      />
    </aside>

    <!-- 模板管理弹窗 -->
    <el-dialog v-model="showTemplateDialog" title="模板管理" width="520px" destroy-on-close>
      <TemplateManager
        :current-pipeline="pipeline.toPipelineJSON()"
        @load="handleLoadTemplate"
        @import-json="
          showTemplateDialog = false;
          handleImport();
        "
      />
    </el-dialog>
  </div>
</template>

<style scoped>
.pipeline-editor {
  display: flex;
  /* 使用 flex:1 撑满父容器（el-main 是 flex column），
     避免 height:100% 在 overflow-y:auto 父容器中解析失败 */
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.editor-left {
  width: 220px;
  min-width: 180px;
  border-right: 1px solid var(--el-border-color-light);
  background-color: var(--el-bg-color);
}
.editor-center {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.editor-right {
  width: 260px;
  min-width: 200px;
  border-left: 1px solid var(--el-border-color-light);
  background-color: var(--el-bg-color);
}
.canvas-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--el-border-color-light);
  background-color: var(--el-bg-color);
  flex-shrink: 0;
}
.toolbar-right {
  display: flex;
  gap: 4px;
}
.flow-canvas {
  flex: 1;
  min-height: 0;
  background-color: var(--el-bg-color-page);
}
.flow-canvas :deep(.vue-flow__pane) {
  cursor: grab;
}
.flow-canvas :deep(.vue-flow__pane:active) {
  cursor: grabbing;
}
.flow-canvas :deep(.vue-flow__node.selected) {
  box-shadow: 0 0 0 2px var(--el-color-primary);
  border-radius: 6px;
}
</style>
