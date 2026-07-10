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
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import {
  Delete, Upload, Download, Check, RefreshLeft, Files,
  VideoPlay, View, FolderOpened,
} from "@element-plus/icons-vue";
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

// ====== 执行状态 ======
const inputFiles = ref<string[]>([]);
const outputDir = ref("");
const executing = ref(false);
const previewing = ref(false);
const execProgress = ref("");
const execCurrentStep = ref(0);
const execTotalSteps = ref(0);
const showPreview = ref(false);
const previewData = ref<any>(null);
let unlistenPipeline: UnlistenFn | null = null;

/** 是否在 Tauri 环境（只有桌面应用才能执行流水线） */
const isTauri = !!(window as any).__TAURI_INTERNALS__;

/** 选择输入文件 */
async function selectInputFiles() {
  if (!isTauri) { ElMessage.warning("请在桌面应用中操作"); return; }
  try {
    const selected = await open({ multiple: true });
    if (!selected) return;
    inputFiles.value = Array.isArray(selected) ? selected : [selected];
  } catch (e) {
    ElMessage.error("选择文件失败: " + String(e));
  }
}

/** 选择输出目录 */
async function selectOutputDir() {
  if (!isTauri) { ElMessage.warning("请在桌面应用中操作"); return; }
  try {
    const selected = await open({ directory: true });
    if (!selected) return;
    outputDir.value = typeof selected === "string" ? selected : selected[0];
  } catch (e) {
    ElMessage.error("选择目录失败: " + String(e));
  }
}

/** 预览（dry-run） */
async function handlePreview() {
  if (!isTauri) { ElMessage.warning("流水线执行需在桌面应用中操作"); return; }
  const json = pipeline.toPipelineJSON();
  if (!json.nodes.length) { ElMessage.warning("请先添加节点"); return; }

  previewing.value = true;
  try {
    const result = await invoke<any>("preview_pipeline", {
      pipelineJson: JSON.stringify(json),
      inputFiles: inputFiles.value,
    });
    previewData.value = result;
    showPreview.value = true;
  } catch (e) {
    ElMessage.error(String(e));
  } finally {
    previewing.value = false;
  }
}

/** 执行流水线 */
async function handleExecute() {
  if (!isTauri) { ElMessage.warning("流水线执行需在桌面应用中操作"); return; }
  if (!outputDir.value) { ElMessage.warning("请选择输出目录"); return; }
  const json = pipeline.toPipelineJSON();
  if (!json.nodes.length) { ElMessage.warning("请先添加节点"); return; }

  executing.value = true;
  execProgress.value = "正在准备执行...";

  try {
    unlistenPipeline = await listen<any>("pipeline-progress", (event) => {
      const p = event.payload;
      execCurrentStep.value = p.currentStep;
      execTotalSteps.value = p.totalSteps;
      execProgress.value = `${p.currentNodeLabel || ""} (${p.currentStep}/${p.totalSteps})`;
    });
  } catch { /* ignore in browser */ }

  try {
    const result = await invoke<any>("run_pipeline", {
      pipelineJson: JSON.stringify(json),
      inputFiles: inputFiles.value,
      outputDir: outputDir.value,
    });
    if (result.success) {
      ElMessage.success("流水线执行完成");
    } else {
      ElMessage.error("执行失败: " + (result.steps?.find((s: any) => !s.success)?.error || "未知错误"));
    }
  } catch (e) {
    ElMessage.error(String(e));
  } finally {
    executing.value = false;
    if (unlistenPipeline) { unlistenPipeline(); unlistenPipeline = null; }
  }
}

function handleCancel() {
  invoke("cancel_batch").catch(() => {});
  executing.value = false;
}

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
  if ((window as any).__TAURI_INTERNALS__) {
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
      <!-- 执行控制栏 -->
      <div class="execution-bar">
        <div class="exec-left">
          <el-tag v-if="inputFiles.length" size="small" type="info">
            {{ inputFiles.length }} 个输入文件
          </el-tag>
          <el-button size="small" :disabled="!isTauri" @click="selectInputFiles">
            <el-icon><Upload /></el-icon>输入文件
          </el-button>
          <el-tag v-if="outputDir" size="small" type="info" class="dir-tag">
            {{ outputDir }}
          </el-tag>
          <el-button size="small" :disabled="!isTauri" @click="selectOutputDir">
            <el-icon><FolderOpened /></el-icon>输出目录
          </el-button>
        </div>
        <div class="exec-right">
          <el-button size="small" :loading="previewing" :disabled="!isTauri" @click="handlePreview">
            <el-icon><View /></el-icon>预览
          </el-button>
          <el-button size="small" type="primary" :loading="executing" :disabled="!isTauri || !outputDir" @click="handleExecute">
            <el-icon><VideoPlay /></el-icon>执行
          </el-button>
          <el-button v-if="executing" size="small" type="warning" @click="handleCancel">取消</el-button>
        </div>
      </div>
      <!-- 执行/预览进度 -->
      <div v-if="executing" class="exec-progress">
        <el-progress :percentage="execTotalSteps > 0 ? Math.round(execCurrentStep / execTotalSteps * 100) : undefined" :indeterminate="execTotalSteps === 0" />
        <span class="progress-text">{{ execProgress || "执行中..." }}</span>
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
    <!-- 预览弹窗 -->
    <el-dialog v-model="showPreview" title="执行预览 (Dry-run)" width="560px" destroy-on-close>
      <div v-if="previewData">
        <el-alert v-if="previewData.hasCycle" type="error" title="流水线中存在环路，无法执行" :closable="false" />
        <el-alert v-else-if="previewData.error" type="warning" :title="previewData.error" :closable="false" />
        <div v-else>
          <p style="margin-bottom: 12px; color: var(--el-text-color-secondary)">
            执行顺序: {{ (previewData.executionOrder || []).join(" → ") }}
          </p>
          <div v-for="(step, i) in previewData.steps || []" :key="i" class="preview-step">
            <h4>
              <el-tag size="small" type="primary">{{ i + 1 }}</el-tag>
              {{ step.nodeLabel || step.nodeId }}
            </h4>
            <p v-if="step.inputFrom">输入来源: {{ step.inputFrom }}</p>
            <p v-if="step.inputFiles?.length">
              → {{ step.inputFiles.length }} 输入 → {{ step.outputFiles?.length || 0 }} 输出
            </p>
            <div v-if="step.outputFiles?.length" class="preview-outputs">
              <span v-for="f in step.outputFiles.slice(0, 5)" :key="f" class="preview-file">
                {{ f.split(/[/\\]/).pop() }}
              </span>
              <span v-if="step.outputFiles.length > 5">...等 {{ step.outputFiles.length }} 个</span>
            </div>
          </div>
        </div>
      </div>
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

/* -- 执行控制栏 -- */
.execution-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background: var(--el-bg-color);
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-shrink: 0;
}
.exec-left, .exec-right {
  display: flex;
  align-items: center;
  gap: 6px;
}
.dir-tag {
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* -- 执行进度 -- */
.exec-progress {
  padding: 6px 12px;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  background: var(--el-color-primary-light-9);
}
.exec-progress .el-progress {
  flex: 1;
}
.progress-text {
  font-size: 12px;
  color: var(--el-color-primary);
  white-space: nowrap;
}

/* -- 预览 -- */
.preview-step {
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 6px;
  padding: 8px 12px;
  margin-bottom: 8px;
}
.preview-step h4 {
  margin: 0 0 4px;
  font-size: 14px;
  display: flex;
  align-items: center;
  gap: 6px;
}
.preview-step p {
  margin: 2px 0;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.preview-outputs {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 4px;
}
.preview-file {
  font-size: 11px;
  padding: 1px 6px;
  background: var(--el-color-success-light-9);
  border-radius: 3px;
  color: var(--el-color-success);
}
</style>
