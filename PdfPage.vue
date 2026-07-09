<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage } from "element-plus";

const activeTab = ref("merge");

// ── 通用文件选择 ──
function selectFiles(accept: string, multiple: boolean): Promise<File[]> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = multiple;
    if (accept) input.accept = accept;
    input.onchange = () => {
      const files = input.files;
      resolve(files ? Array.from(files) : []);
      input.remove();
    };
    input.click();
  });
}

function getPath(file: File): string {
  return (file as any).path || file.name;
}

// ====== 合并 ======
const mergeFiles = ref<string[]>([]);
const mergeOutput = ref("");

async function selectMergeFiles() {
  const files = await selectFiles(".pdf", true);
  mergeFiles.value = files.map(getPath);
}

function removeMergeFile(i: number) { mergeFiles.value.splice(i, 1); }

async function mergePdfs() {
  if (mergeFiles.value.length < 2 || !mergeOutput.value) return;
  try {
    const msg = await invoke<string>("merge_pdfs", { files: mergeFiles.value, outputPath: mergeOutput.value });
    ElMessage.success(msg);
  } catch (e) { ElMessage.error(String(e)); }
}

// ====== 拆分 ======
const splitFile = ref("");
const splitRanges = ref(["1-3", "4-"]);
const splitOutputDir = ref("");

async function selectSplitFile() {
  const files = await selectFiles(".pdf", false);
  if (files.length > 0) splitFile.value = getPath(files[0]);
}

function addRange() { splitRanges.value.push(""); }
function removeRange(i: number) { splitRanges.value.splice(i, 1); }

async function splitPdf() {
  if (!splitFile.value || splitRanges.value.length === 0 || !splitOutputDir.value) return;
  try {
    const results = await invoke<string[]>("split_pdf", {
      file: splitFile.value,
      ranges: splitRanges.value.filter(r => r.trim()),
      outputDir: splitOutputDir.value,
    });
    ElMessage.success(`拆分完成，生成 ${results.length} 个文件`);
  } catch (e) { ElMessage.error(String(e)); }
}

// ====== 压缩 ======
const compressFile = ref("");
const compressOutput = ref("");

async function selectCompressFile() {
  const files = await selectFiles(".pdf", false);
  if (files.length > 0) {
    const path = getPath(files[0]);
    compressFile.value = path;
    compressOutput.value = path.replace(/\.pdf$/i, "_compressed.pdf");
  }
}

async function compressPdf() {
  if (!compressFile.value || !compressOutput.value) return;
  try {
    const msg = await invoke<string>("compress_pdf", { file: compressFile.value, outputPath: compressOutput.value });
    ElMessage.success(msg);
  } catch (e) { ElMessage.error(String(e)); }
}
</script>

<template>
  <div class="page-container">
    <h2>PDF 工具</h2>
    <p class="page-desc">合并、拆分、压缩 PDF 文件</p>

    <el-tabs v-model="activeTab" class="pdf-tabs">
      <!-- 合并 -->
      <el-tab-pane label="合并" name="merge">
        <section class="section">
          <h3>选择 PDF 文件</h3>
          <div class="drop-zone" @click="selectMergeFiles">
            <p>点击选择多个 PDF 文件（按选择顺序合并）</p>
          </div>
          <div v-if="mergeFiles.length > 0" class="file-list">
            <div v-for="(f, i) in mergeFiles" :key="i" class="file-item">
              <span>{{ f.split(/[/\\]/).pop() }}</span>
              <el-button size="small" type="danger" text @click="removeMergeFile(i)">移除</el-button>
            </div>
            <p class="file-count">共 {{ mergeFiles.length }} 个文件</p>
          </div>
          <el-form label-width="100px" class="form-section">
            <el-form-item label="输出路径">
              <el-input v-model="mergeOutput" placeholder="例如: D:/merged.pdf" />
            </el-form-item>
          </el-form>
          <el-button type="primary" :disabled="mergeFiles.length < 2 || !mergeOutput" @click="mergePdfs">合并 PDF</el-button>
        </section>
      </el-tab-pane>

      <!-- 拆分 -->
      <el-tab-pane label="拆分" name="split">
        <section class="section">
          <h3>选择 PDF 文件</h3>
          <div class="drop-zone" @click="selectSplitFile">
            <p>{{ splitFile ? splitFile.split(/[/\\]/).pop() : '点击选择 PDF 文件' }}</p>
          </div>
          <h3>页码范围</h3>
          <p class="hint">每行一个范围，例如: 1-5, 6-10, 11- （留空表示到末尾）</p>
          <div v-for="(_r, i) in splitRanges" :key="i" class="range-row">
            <el-input v-model="splitRanges[i]" placeholder="例如: 1-5" style="width: 200px" />
            <el-button size="small" type="danger" text @click="removeRange(i)">移除</el-button>
          </div>
          <el-button size="small" @click="addRange">+ 添加范围</el-button>
          <el-form label-width="100px" class="form-section">
            <el-form-item label="输出目录">
              <el-input v-model="splitOutputDir" placeholder="例如: D:/split_output" />
            </el-form-item>
          </el-form>
          <el-button type="primary" :disabled="!splitFile || splitRanges.length === 0 || !splitOutputDir" @click="splitPdf">拆分 PDF</el-button>
        </section>
      </el-tab-pane>

      <!-- 压缩 -->
      <el-tab-pane label="压缩" name="compress">
        <section class="section">
          <h3>选择 PDF 文件</h3>
          <div class="drop-zone" @click="selectCompressFile">
            <p>{{ compressFile ? compressFile.split(/[/\\]/).pop() : '点击选择 PDF 文件' }}</p>
          </div>
          <el-form label-width="100px" class="form-section">
            <el-form-item label="输出路径">
              <el-input v-model="compressOutput" placeholder="例如: D:/compressed.pdf" />
            </el-form-item>
          </el-form>
          <el-button type="primary" :disabled="!compressFile || !compressOutput" @click="compressPdf">压缩 PDF</el-button>
        </section>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<style scoped>
.page-container { padding: 24px; max-width: 800px; }
h2 { margin: 0 0 4px; }
.page-desc { color: var(--el-text-color-secondary); font-size: 14px; margin: 0 0 24px; }
.pdf-tabs { margin-top: 16px; }
.section { background: var(--el-bg-color); border: 1px solid var(--el-border-color-light); border-radius: 8px; padding: 16px 20px; margin-bottom: 16px; }
.section h3 { margin: 0 0 12px; font-size: 15px; }
.drop-zone { border: 2px dashed var(--el-border-color); border-radius: 8px; padding: 30px; text-align: center; cursor: pointer; color: var(--el-text-color-secondary); }
.drop-zone:hover { border-color: var(--el-color-primary); color: var(--el-color-primary); }
.file-list { margin-top: 12px; }
.file-item { display: flex; justify-content: space-between; padding: 4px 0; font-size: 13px; border-bottom: 1px solid var(--el-border-color-lighter); }
.file-count { font-size: 12px; color: var(--el-text-color-secondary); margin-top: 8px; }
.form-section { margin-top: 16px; }
.hint { font-size: 12px; color: var(--el-text-color-secondary); margin: -8px 0 12px; }
.range-row { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
</style>
