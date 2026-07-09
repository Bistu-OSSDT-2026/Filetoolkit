<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage } from "element-plus";

const inputFile = ref("");
const audioFormat = ref("mp3");
const audioBitrate = ref("192k");
const outputPath = ref("");

function selectFile() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = "video/*,audio/*";
  input.onchange = (e) => {
    const files = (e.target as HTMLInputElement).files;
    if (files && files[0]) {
      const path = (files[0] as any).path || files[0].name;
      inputFile.value = path;
      // 自动生成输出路径
      const name = path.replace(/\.[^/.]+$/, "");
      outputPath.value = `${name}.${audioFormat.value}`;
    }
  };
  input.click();
}

async function extractAudio() {
  if (!inputFile.value || !outputPath.value) return;
  try {
    const msg = await invoke<string>("extract_audio", {
      app: null, input: inputFile.value,
      format: audioFormat.value, bitrate: audioBitrate.value,
      output: outputPath.value,
    });
    ElMessage.success(msg);
  } catch (e) { ElMessage.error(String(e)); }
}
</script>

<template>
  <div class="page-container">
    <h2>音频提取</h2>
    <p class="page-desc">从视频文件中提取音频（需要 ffmpeg）</p>

    <section class="section">
      <div class="drop-zone" @click="selectFile">
        <p>{{ inputFile ? inputFile.split(/[/\\]/).pop() : '点击选择视频或音频文件' }}</p>
      </div>

      <el-form label-width="100px" class="form-section">
        <el-form-item label="输出格式">
          <el-select v-model="audioFormat">
            <el-option label="MP3" value="mp3" />
            <el-option label="AAC" value="aac" />
            <el-option label="FLAC (无损)" value="flac" />
            <el-option label="OGG" value="ogg" />
            <el-option label="WAV" value="wav" />
          </el-select>
        </el-form-item>
        <el-form-item label="比特率">
          <el-select v-model="audioBitrate">
            <el-option label="128 kbps" value="128k" />
            <el-option label="192 kbps (推荐)" value="192k" />
            <el-option label="256 kbps" value="256k" />
            <el-option label="320 kbps" value="320k" />
          </el-select>
        </el-form-item>
        <el-form-item label="输出路径">
          <el-input v-model="outputPath" placeholder="例如: D:/audio.mp3" />
        </el-form-item>
      </el-form>

      <el-button type="primary" :disabled="!inputFile || !outputPath" @click="extractAudio">提取音频</el-button>
    </section>
  </div>
</template>

<style scoped>
.page-container { padding: 24px; max-width: 800px; }
h2 { margin: 0 0 4px; }
.page-desc { color: var(--el-text-color-secondary); font-size: 14px; margin: 0 0 24px; }
.section { background: var(--el-bg-color); border: 1px solid var(--el-border-color-light); border-radius: 8px; padding: 16px 20px; }
.drop-zone { border: 2px dashed var(--el-border-color); border-radius: 8px; padding: 30px; text-align: center; cursor: pointer; color: var(--el-text-color-secondary); margin-bottom: 16px; }
.drop-zone:hover { border-color: var(--el-color-primary); color: var(--el-color-primary); }
.form-section { margin-top: 8px; }
</style>
