<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ElMessage, ElMessageBox } from "element-plus";

const { t } = useI18n();

// ====== 状态 ======
const folderPath = ref("");
const isScanning = ref(false);
const isDeleting = ref(false);
const duplicateGroups = ref<string[][]>([]);
const checkedFiles = ref<Set<string>>(new Set());

// ====== 选择文件夹（Tauri 原生对话框）======
async function selectFolder() {
  const selected = await open({ directory: true });
  if (!selected) return;
  const dir = typeof selected === "string" ? selected : selected[0];
  if (dir) {
    folderPath.value = dir.replace(/\\/g, "/");
  }
}

// ====== 扫描 ======
async function scanDuplicates() {
  if (!folderPath.value.trim()) {
    ElMessage.warning("请输入或选择要扫描的文件夹");
    return;
  }

  isScanning.value = true;
  duplicateGroups.value = [];
  checkedFiles.value = new Set();

  try {
    const res = await invoke<{
      success: boolean;
      msg: string;
      duplicate_groups: string[][];
    }>("scan_duplicate_files", {
      folderPath: folderPath.value.trim(),
    });

    if (res.success && res.duplicate_groups) {
      duplicateGroups.value = res.duplicate_groups;
      // 默认勾选每组中除第一个外的所有文件
      for (const group of res.duplicate_groups) {
        for (let i = 1; i < group.length; i++) {
          checkedFiles.value.add(group[i]);
        }
      }
      ElMessage.success(`扫描完成，发现 ${res.duplicate_groups.length} 组重复文件`);
    } else {
      ElMessage.warning(res.msg || "未发现重复文件");
    }
  } catch (e) {
    ElMessage.error(String(e));
  } finally {
    isScanning.value = false;
  }
}

// ====== 勾选控制 ======
function toggleFile(file: string) {
  const newSet = new Set(checkedFiles.value);
  if (newSet.has(file)) {
    newSet.delete(file);
  } else {
    newSet.add(file);
  }
  checkedFiles.value = newSet;
}

function isChecked(file: string): boolean {
  return checkedFiles.value.has(file);
}

function getFileName(path: string): string {
  return path.split(/[/\\]/).pop() || path;
}

// ====== 删除 ======
function getSelectedFiles(): string[] {
  return duplicateGroups.value.flat().filter((f) => checkedFiles.value.has(f));
}

async function deleteSelected() {
  const delPaths = getSelectedFiles();
  if (delPaths.length === 0) {
    ElMessage.warning("请勾选要删除的重复文件");
    return;
  }

  try {
    await ElMessageBox.confirm(
      `确定删除选中的 ${delPaths.length} 个重复文件吗？此操作不可撤销。`,
      "确认删除",
      { confirmButtonText: "删除", cancelButtonText: "取消", type: "warning" },
    );
  } catch {
    return; // 用户取消
  }

  isDeleting.value = true;

  try {
    const res = await invoke<{
      success: boolean;
      msg: string;
      delete_fail: string[];
    }>("delete_duplicate", {
      delPaths: delPaths,
    });

    if (res.success) {
      ElMessage.success(res.msg);
      // 移除已删除的文件
      const deletedSet = new Set(delPaths.filter((p) => !res.delete_fail?.includes(p)));
      duplicateGroups.value = duplicateGroups.value
        .map((group) => group.filter((f) => !deletedSet.has(f)))
        .filter((group) => group.length > 0);
      // 清除已删除文件的选中状态
      for (const f of delPaths) {
        checkedFiles.value.delete(f);
      }
    } else {
      ElMessage.error(res.msg);
    }

    if (res.delete_fail && res.delete_fail.length > 0) {
      ElMessage.warning(`${res.delete_fail.length} 个文件删除失败`);
    }
  } catch (e) {
    ElMessage.error(String(e));
  } finally {
    isDeleting.value = false;
  }
}
</script>

<template>
  <div class="page-container">
    <h2>{{ t("dedup.title") }}</h2>
    <p class="page-desc">{{ t("dedup.desc") }}</p>

    <section class="section">
      <h3>{{ t("dedup.selectDir") }}</h3>
      <div class="folder-row">
        <el-input v-model="folderPath" placeholder="例如: D:/我的文件" style="flex: 1" />
        <el-button @click="selectFolder">{{ t("dedup.selectFolder") }}</el-button>
      </div>
      <el-button type="primary" size="large" :disabled="!folderPath.trim() || isScanning" :loading="isScanning" style="margin-top: 12px" @click="scanDuplicates">
        {{ isScanning ? t("dedup.scanning") : t("dedup.scan") }}
      </el-button>
    </section>

    <section v-if="duplicateGroups.length > 0" class="section">
      <div class="result-header">
        <h3>{{ t("dedup.result") }}</h3>
        <span class="group-count">{{ t("dedup.totalGroups", { count: duplicateGroups.length }) }}</span>
      </div>
      <div v-for="(group, gi) in duplicateGroups" :key="gi" class="group-card">
        <div class="group-title">
          <el-tag size="small" type="warning">{{ t("dedup.group", { index: gi + 1 }) }}</el-tag>
          <span class="group-hint">{{ t("dedup.groupHint") }}</span>
        </div>
        <div v-for="(file, fi) in group" :key="fi" class="file-row" :class="{ 'keep-file': fi === 0 }">
          <el-checkbox v-if="fi > 0" :model-value="isChecked(file)" @change="() => toggleFile(file)" />
          <span v-else class="keep-badge">{{ t("dedup.keep") }}</span>
          <span class="file-name" :title="file">{{ getFileName(file) }}</span>
          <span class="file-path">{{ file }}</span>
        </div>
      </div>
      <div class="delete-actions">
        <span class="selected-count">{{ t("dedup.selectedCount", { count: getSelectedFiles().length }) }}</span>
        <el-button type="danger" :disabled="getSelectedFiles().length === 0 || isDeleting" :loading="isDeleting" @click="deleteSelected">
          {{ t("dedup.deleteSelected") }}
        </el-button>
      </div>
    </section>

    <section v-if="!isScanning && folderPath && duplicateGroups.length === 0" class="section">
      <el-alert :title="t('dedup.noDuplicates')" type="success" show-icon />
    </section>
  </div>
</template>

<style scoped>
.page-container {
  padding: 24px;
  max-width: 800px;
}
h2 {
  margin: 0 0 4px;
}
.page-desc {
  color: var(--el-text-color-secondary);
  font-size: 14px;
  margin: 0 0 24px;
}
.section {
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
  padding: 16px 20px;
  margin-bottom: 16px;
}
.section h3 {
  margin: 0 0 12px;
  font-size: 15px;
}
.folder-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.result-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}
.result-header h3 {
  margin: 0;
}
.group-count {
  font-size: 13px;
  color: var(--el-text-color-secondary);
}

.group-card {
  border: 1px solid var(--el-border-color-light);
  border-radius: 6px;
  margin-bottom: 12px;
  overflow: hidden;
}
.group-title {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--el-color-warning-light-9);
  border-bottom: 1px solid var(--el-border-color-lighter);
  font-size: 13px;
}
.group-hint {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  font-size: 13px;
}
.file-row:last-child {
  border-bottom: none;
}
.file-row.keep-file {
  background: var(--el-color-success-light-9);
}
.keep-badge {
  display: inline-block;
  font-size: 11px;
  color: #67c23a;
  font-weight: 600;
  width: 36px;
  text-align: center;
}
.file-name {
  font-weight: 500;
  white-space: nowrap;
}
.file-path {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.delete-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--el-border-color-light);
}
.selected-count {
  font-size: 14px;
  color: var(--el-text-color-regular);
}
</style>
