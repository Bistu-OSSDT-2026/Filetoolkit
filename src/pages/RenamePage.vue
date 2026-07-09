<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ElMessage } from "element-plus";

const { t } = useI18n();

// ====== 状态 ======
const filePaths = ref("");
const prefix = ref("");
const startNum = ref(1);
const suffix = ref("");
const isRunning = ref(false);
const resultMsg = ref("");
const failList = ref<string[]>([]);

// ====== 选择文件（Tauri 原生对话框）======
async function selectFiles() {
  const selected = await open({ multiple: true });
  if (!selected) return;
  const paths = Array.isArray(selected) ? selected : [selected];
  if (paths.length > 0) {
    filePaths.value = paths.join("\n");
  }
}

// ====== 执行重命名 ======
async function renameFiles() {
  const paths = filePaths.value
    .split("\n")
    .map((s) => s.trim())
    .filter(Boolean);
  if (paths.length === 0) {
    ElMessage.warning("请选择或输入文件路径");
    return;
  }

  isRunning.value = true;
  resultMsg.value = "";
  failList.value = [];

  try {
    const res = await invoke<{
      success: boolean;
      msg: string;
      fail_list: string[];
    }>("rename_files", {
      filePaths: paths,
      prefix: prefix.value,
      startNum: startNum.value,
      suffix: suffix.value,
    });

    resultMsg.value = res.msg;
    if (res.fail_list && res.fail_list.length > 0) {
      failList.value = res.fail_list;
    }

    if (res.success) {
      ElMessage.success(res.msg);
    } else {
      ElMessage.error(res.msg);
    }
  } catch (e) {
    ElMessage.error(String(e));
  } finally {
    isRunning.value = false;
  }
}
</script>

<template>
  <div class="page-container">
    <h2>{{ t("rename.title") }}</h2>
    <p class="page-desc">{{ t("rename.desc") }}</p>

    <section class="section">
      <h3>{{ t("rename.selectFiles") }}</h3>
      <div class="drop-zone" @click="selectFiles">
        <p>{{ t("rename.clickSelect") }}</p>
      </div>

      <el-form label-width="100px" class="form-section">
        <el-form-item :label="t('rename.filePaths')">
          <el-input v-model="filePaths" type="textarea" :rows="4" :placeholder="t('rename.filePathPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('rename.prefix')">
          <el-input v-model="prefix" :placeholder="t('rename.prefixPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('rename.startNum')">
          <el-input-number v-model="startNum" :min="0" :max="99999" />
          <span class="form-hint">{{ t("rename.startNumHint") }}</span>
        </el-form-item>
        <el-form-item :label="t('rename.suffix')">
          <el-input v-model="suffix" :placeholder="t('rename.suffixPlaceholder')" />
        </el-form-item>
      </el-form>

      <el-button type="primary" size="large" :disabled="!filePaths.trim() || isRunning" :loading="isRunning" @click="renameFiles">
        {{ t("rename.startRename") }}
      </el-button>
    </section>

    <!-- 结果 -->
    <section v-if="resultMsg" class="section">
      <h3>{{ t("rename.result") }}</h3>
      <el-alert :title="resultMsg" :type="failList.length > 0 ? 'warning' : 'success'" show-icon />
      <div v-if="failList.length > 0" class="fail-section">
        <h4>{{ t("rename.failedFiles", { count: failList.length }) }}:</h4>
        <div v-for="(f, i) in failList" :key="i" class="fail-item">{{ f }}</div>
      </div>
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
.drop-zone {
  border: 2px dashed var(--el-border-color);
  border-radius: 8px;
  padding: 24px;
  text-align: center;
  cursor: pointer;
  color: var(--el-text-color-secondary);
  margin-bottom: 16px;
}
.drop-zone:hover {
  border-color: var(--el-color-primary);
  color: var(--el-color-primary);
}
.form-section {
  margin-top: 8px;
}
.form-hint {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-left: 8px;
}
.fail-section {
  margin-top: 12px;
}
.fail-section h4 {
  margin: 0 0 8px;
  font-size: 14px;
  color: var(--el-color-warning);
}
.fail-item {
  font-size: 13px;
  padding: 4px 0;
  color: var(--el-text-color-secondary);
}
</style>
