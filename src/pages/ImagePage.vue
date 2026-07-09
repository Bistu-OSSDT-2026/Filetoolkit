<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useBatchTask } from "../composables/useBatchTask";
import FileDropZone from "../components/FileDropZone.vue";
import TaskProgress from "../components/TaskProgress.vue";
import ResultList from "../components/ResultList.vue";
import type { ResultItem } from "../components/ResultList.vue";

// ====== 状态 ======
const filePaths = ref<string[]>([]);
const format = ref("jpg");
const quality = ref(80);
const maxWidth = ref<number | undefined>(undefined);
const maxHeight = ref<number | undefined>(undefined);
const outputDir = ref("");
const results = ref<ResultItem[]>([]);

const { run, cancel, progress, status, message } = useBatchTask("compress_images");
const { t } = useI18n();

// ====== 文件选择 ======
function onFilesSelected(paths: string[]) {
  filePaths.value = [...filePaths.value, ...paths];
}

function removeFile(index: number) {
  filePaths.value.splice(index, 1);
}

function clearFiles() {
  filePaths.value = [];
  results.value = [];
}

// ====== 执行 ======
async function startCompress() {
  if (filePaths.value.length === 0 || !outputDir.value.trim()) return;
  results.value = [];

  try {
    const res = await run<any[]>("compress_images", {
      files: filePaths.value,
      quality: quality.value,
      format: format.value,
      maxWidth: maxWidth.value && maxWidth.value > 0 ? maxWidth.value : null,
      maxHeight: maxHeight.value && maxHeight.value > 0 ? maxHeight.value : null,
      outputDir: outputDir.value.trim(),
    });

    results.value = res.map((r: any) => ({
      name: r.inputPath.split(/[/\\]/).pop() || r.inputPath,
      status: r.status.type === "completed" ? ("success" as const) : ("error" as const),
      originalSize: r.originalSize ?? undefined,
      newSize: r.newSize ?? undefined,
      error: r.status.type === "failed" ? r.status.message : undefined,
    }));
  } catch (e) {
    console.error(e);
  }
}
</script>

<template>
  <div class="page-container">
    <h2>{{ t("image.title") }}</h2>
    <p class="page-desc">{{ t("image.desc") }}</p>

    <!-- 文件选择 -->
    <section class="section">
      <h3>{{ t("image.selectFiles") }}</h3>
      <FileDropZone accept="image/*" @files-selected="onFilesSelected" />
      <div v-if="filePaths.length > 0" class="file-list">
        <div v-for="(f, i) in filePaths" :key="i" class="file-item">
          <span>{{ f }}</span>
          <el-button size="small" type="danger" text @click="removeFile(i)">{{ t("image.removeFile") }}</el-button>
        </div>
        <el-button size="small" @click="clearFiles">{{ t("image.clearAll") }}</el-button>
      </div>
    </section>

    <!-- 参数 -->
    <section class="section">
      <h3>{{ t("image.outputSettings") }}</h3>
      <el-form label-width="110px" size="default">
        <el-form-item :label="t('image.outputFormat')">
          <el-select v-model="format">
            <el-option label="JPEG" value="jpg" />
            <el-option :label="t('image.noLimit') + ' PNG'" value="png" />
            <el-option :label="t('image.noLimit') + ' WebP'" value="webp" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('image.quality')">
          <el-slider v-model="quality" :min="1" :max="100" show-input style="width: 300px" />
        </el-form-item>
        <el-form-item :label="t('image.maxWidth')">
          <el-input-number v-model="maxWidth" :min="0" :placeholder="t('image.noLimit')" clearable />
        </el-form-item>
        <el-form-item :label="t('image.maxHeight')">
          <el-input-number v-model="maxHeight" :min="0" :placeholder="t('image.noLimit')" clearable />
        </el-form-item>
        <el-form-item :label="t('image.outputDir')">
          <el-input v-model="outputDir" placeholder="例如: D:/images/output" style="width: 400px" />
          <div class="form-hint">{{ t("image.outputDirHint") }}</div>
        </el-form-item>
      </el-form>
    </section>

    <!-- 执行 -->
    <div class="actions">
      <el-button
        type="primary"
        size="large"
        :disabled="filePaths.length === 0 || !outputDir || status === 'running'"
        @click="startCompress"
      >
        {{ status === "running" ? t("image.processing") : t("image.startCompress") }}
      </el-button>
      <el-button v-if="status === 'running'" type="warning" @click="cancel">取消</el-button>
    </div>

    <!-- 进度 -->
    <TaskProgress
      v-if="status !== 'idle'"
      :progress="progress"
      :status="status"
      :message="message"
      @cancel="cancel"
    />

    <!-- 结果 -->
    <ResultList :items="results" />
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
.file-list {
  margin-top: 12px;
}
.file-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 0;
  font-size: 13px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}
.actions {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.form-hint {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-top: 4px;
}
</style>
