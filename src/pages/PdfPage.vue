<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ElMessage } from "element-plus";

const { t } = useI18n();

const activeTab = ref("merge");

// ── 通用文件选择（Tauri 原生对话框，返回真实路径）──
async function selectFiles(acceptExt: string, multiple: boolean): Promise<string[]> {
  const filters = acceptExt
    .split(",")
    .map((s) => {
      const ext = s.trim().replace(/^\./, "");
      return { name: ext.toUpperCase(), extensions: [ext.toLowerCase()] };
    });

  const selected = await open({ multiple, filters });
  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

// ====== 合并 ======
const mergeFiles = ref<string[]>([]);
const mergeOutput = ref("");

async function selectMergeFiles() {
  mergeFiles.value = await selectFiles(".pdf", true);
}

function removeMergeFile(i: number) {
  mergeFiles.value.splice(i, 1);
}

async function mergePdfs() {
  if (mergeFiles.value.length < 2 || !mergeOutput.value) return;
  try {
    const msg = await invoke<string>("merge_pdfs", {
      files: mergeFiles.value,
      outputPath: mergeOutput.value,
    });
    ElMessage.success(msg);
  } catch (e) {
    ElMessage.error(String(e));
  }
}

// ====== 拆分 ======
const splitFile = ref("");
const splitRanges = ref(["1-3", "4-"]);
const splitOutputDir = ref("");

async function selectSplitFile() {
  const paths = await selectFiles(".pdf", false);
  if (paths.length > 0) splitFile.value = paths[0];
}

function addRange() {
  splitRanges.value.push("");
}
function removeRange(i: number) {
  splitRanges.value.splice(i, 1);
}

async function splitPdf() {
  if (!splitFile.value || splitRanges.value.length === 0 || !splitOutputDir.value) return;
  try {
    const results = await invoke<string[]>("split_pdf", {
      file: splitFile.value,
      ranges: splitRanges.value.filter((r) => r.trim()),
      outputDir: splitOutputDir.value,
    });
    ElMessage.success(`拆分完成，生成 ${results.length} 个文件`);
  } catch (e) {
    ElMessage.error(String(e));
  }
}

// ====== 压缩 ======
const compressFile = ref("");
const compressOutput = ref("");

async function selectCompressFile() {
  const paths = await selectFiles(".pdf", false);
  if (paths.length > 0) {
    compressFile.value = paths[0];
    compressOutput.value = paths[0].replace(/\.pdf$/i, "_compressed.pdf");
  }
}

async function compressPdf() {
  if (!compressFile.value || !compressOutput.value) return;
  try {
    const msg = await invoke<string>("compress_pdf", {
      file: compressFile.value,
      outputPath: compressOutput.value,
    });
    ElMessage.success(msg);
  } catch (e) {
    ElMessage.error(String(e));
  }
}
</script>

<template>
  <div class="page-container">
    <h2>{{ t("pdf.title") }}</h2>
    <p class="page-desc">{{ t("pdf.desc") }}</p>

    <el-tabs v-model="activeTab" class="pdf-tabs">
      <!-- 合并 -->
      <el-tab-pane :label="t('pdf.merge')" name="merge">
        <section class="section">
          <h3>{{ t("pdf.selectPdfFiles") }}</h3>
          <div class="drop-zone" @click="selectMergeFiles">
            <p>{{ t("pdf.clickSelectMultiple") }}</p>
          </div>
          <div v-if="mergeFiles.length > 0" class="file-list">
            <div v-for="(f, i) in mergeFiles" :key="i" class="file-item">
              <span>{{ f.split(/[/\\]/).pop() }}</span>
              <el-button size="small" type="danger" text @click="removeMergeFile(i)">
                {{ t("common.remove") }}
              </el-button>
            </div>
            <p class="file-count">{{ t("pdf.totalFiles", { count: mergeFiles.length }) }}</p>
          </div>
          <el-form label-width="100px" class="form-section">
            <el-form-item :label="t('pdf.outputPath')">
              <el-input v-model="mergeOutput" placeholder="例如: D:/merged.pdf" />
            </el-form-item>
          </el-form>
          <el-button
            type="primary"
            :disabled="mergeFiles.length < 2 || !mergeOutput"
            @click="mergePdfs"
          >
            {{ t("pdf.mergePdf") }}
          </el-button>
        </section>
      </el-tab-pane>

      <!-- 拆分 -->
      <el-tab-pane :label="t('pdf.split')" name="split">
        <section class="section">
          <h3>{{ t("pdf.selectPdfFiles") }}</h3>
          <div class="drop-zone" @click="selectSplitFile">
            <p>{{ splitFile ? splitFile.split(/[/\\]/).pop() : t("pdf.clickSelectPdf") }}</p>
          </div>
          <h3>{{ t("pdf.pageRanges") }}</h3>
          <p class="hint">{{ t("pdf.pageRangeHint") }}</p>
          <div v-for="(_r, i) in splitRanges" :key="i" class="range-row">
            <el-input v-model="splitRanges[i]" placeholder="例如: 1-5" style="width: 200px" />
            <el-button size="small" type="danger" text @click="removeRange(i)">{{ t("common.remove") }}</el-button>
          </div>
          <el-button size="small" @click="addRange">{{ t("pdf.addRange") }}</el-button>
          <el-form label-width="100px" class="form-section">
            <el-form-item :label="t('pdf.outputDir')">
              <el-input v-model="splitOutputDir" placeholder="例如: D:/split_output" />
            </el-form-item>
          </el-form>
          <el-button
            type="primary"
            :disabled="!splitFile || splitRanges.length === 0 || !splitOutputDir"
            @click="splitPdf"
          >
            {{ t("pdf.splitPdf") }}
          </el-button>
        </section>
      </el-tab-pane>

      <!-- 压缩 -->
      <el-tab-pane :label="t('pdf.compress')" name="compress">
        <section class="section">
          <h3>{{ t("pdf.selectPdfFiles") }}</h3>
          <div class="drop-zone" @click="selectCompressFile">
            <p>{{ compressFile ? compressFile.split(/[/\\]/).pop() : t("pdf.clickSelectPdf") }}</p>
          </div>
          <el-form label-width="100px" class="form-section">
            <el-form-item :label="t('pdf.outputPath')">
              <el-input v-model="compressOutput" placeholder="例如: D:/compressed.pdf" />
            </el-form-item>
          </el-form>
          <el-button
            type="primary"
            :disabled="!compressFile || !compressOutput"
            @click="compressPdf"
          >
            {{ t("pdf.compressPdf") }}
          </el-button>
        </section>
      </el-tab-pane>
    </el-tabs>
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
.pdf-tabs {
  margin-top: 16px;
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
  padding: 30px;
  text-align: center;
  cursor: pointer;
  color: var(--el-text-color-secondary);
}
.drop-zone:hover {
  border-color: var(--el-color-primary);
  color: var(--el-color-primary);
}
.file-list {
  margin-top: 12px;
}
.file-item {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
  font-size: 13px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}
.file-count {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-top: 8px;
}
.form-section {
  margin-top: 16px;
}
.hint {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin: -8px 0 12px;
}
.range-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
</style>
