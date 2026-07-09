<script setup lang="ts">
import { ElMessage, ElMessageBox } from "element-plus";
import { Files, Upload, Download, Delete, Plus } from "@element-plus/icons-vue";
import { useTemplates, type TemplateEntry } from "../composables/useTemplates";
import type { Pipeline } from "../pipeline/types";

const emit = defineEmits<{
  (e: "load", pipeline: Pipeline): void;
  (e: "import-json"): void;
}>();

const props = defineProps<{
  /** 当前画布的流水线数据（用于另存为模板） */
  currentPipeline: Pipeline;
}>();

const { templates, selectedTemplateId, saveAsTemplate, deleteTemplate, exportTemplate } =
  useTemplates();

// ========== 加载模板到画布 ==========
function handleLoad(entry: TemplateEntry) {
  selectedTemplateId.value = entry.id;
  emit("load", JSON.parse(JSON.stringify(entry.pipeline)));
  ElMessage.success(`已加载模板: ${entry.name}`);
}

// ========== 另存为模板 ==========
function handleSaveAs() {
  const entry = saveAsTemplate(props.currentPipeline);
  ElMessage.success(`已保存模板: ${entry.name}`);
}

// ========== 删除用户模板 ==========
function handleDelete(entry: TemplateEntry) {
  ElMessageBox.confirm(`确定删除模板「${entry.name}」？`, "确认", {
    confirmButtonText: "删除",
    cancelButtonText: "取消",
    type: "warning",
  })
    .then(() => {
      deleteTemplate(entry.id);
      ElMessage.success("模板已删除");
    })
    .catch(() => {});
}
</script>

<template>
  <div class="template-manager">
    <div class="tm-header">
      <h3>
        <el-icon><Files /></el-icon> 模板管理
      </h3>
      <div class="tm-actions">
        <el-button size="small" type="primary" @click="handleSaveAs">
          <el-icon><Plus /></el-icon>另存为模板
        </el-button>
        <el-button size="small" @click="emit('import-json')">
          <el-icon><Upload /></el-icon>导入 JSON
        </el-button>
      </div>
    </div>

    <el-divider style="margin: 8px 0" />

    <!-- 内置模板 -->
    <div v-if="templates.filter((t) => t.source === 'builtin').length" class="tm-section">
      <h4 class="tm-section-title">内置模板</h4>
      <div
        v-for="t in templates.filter((t) => t.source === 'builtin')"
        :key="t.id"
        class="tm-item"
        :class="{ selected: selectedTemplateId === t.id }"
      >
        <div class="tm-item-info">
          <span class="tm-item-name">{{ t.name }}</span>
          <span class="tm-item-desc">{{ t.description }}</span>
        </div>
        <div class="tm-item-actions">
          <el-button size="small" text type="primary" @click="handleLoad(t)">加载</el-button>
          <el-button size="small" text @click="exportTemplate(t)">
            <el-icon><Download /></el-icon>
          </el-button>
        </div>
      </div>
    </div>

    <!-- 用户模板 -->
    <div v-if="templates.filter((t) => t.source === 'user').length" class="tm-section">
      <h4 class="tm-section-title">我的模板</h4>
      <div
        v-for="t in templates.filter((t) => t.source === 'user')"
        :key="t.id"
        class="tm-item"
        :class="{ selected: selectedTemplateId === t.id }"
      >
        <div class="tm-item-info">
          <span class="tm-item-name">{{ t.name }}</span>
          <span class="tm-item-desc">{{ t.description }}</span>
        </div>
        <div class="tm-item-actions">
          <el-button size="small" text type="primary" @click="handleLoad(t)">加载</el-button>
          <el-button size="small" text @click="exportTemplate(t)">
            <el-icon><Download /></el-icon>
          </el-button>
          <el-button size="small" text type="danger" @click="handleDelete(t)">
            <el-icon><Delete /></el-icon>
          </el-button>
        </div>
      </div>
    </div>

    <el-empty v-if="templates.length === 0" description="暂无模板" :image-size="60" />
  </div>
</template>

<style scoped>
.template-manager {
  padding: 4px 0;
}

.tm-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.tm-header h3 {
  margin: 0;
  font-size: 15px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.tm-actions {
  display: flex;
  gap: 4px;
}

.tm-section {
  margin-bottom: 12px;
}

.tm-section-title {
  margin: 8px 0 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--el-text-color-secondary);
  text-transform: uppercase;
}

.tm-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  margin-bottom: 4px;
  border: 1px solid var(--el-border-color-light);
  border-radius: 6px;
  transition: border-color 0.15s;
}

.tm-item:hover {
  border-color: var(--el-color-primary-light-5);
}

.tm-item.selected {
  border-color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-9);
}

.tm-item-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.tm-item-name {
  font-size: 13px;
  font-weight: 500;
}

.tm-item-desc {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tm-item-actions {
  display: flex;
  gap: 2px;
  flex-shrink: 0;
  margin-left: 8px;
}
</style>
