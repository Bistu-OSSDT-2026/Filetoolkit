<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ElMessage } from "element-plus";

const { t } = useI18n();

const activeTab = ref("cut");

// ====== ffmpeg 检测 ======
const ffmpegOk = ref(true);
const ffmpegMsg = ref("");
const gpuEncoders = ref<string[]>([]);
const gpuChecked = ref(false);

onMounted(async () => {
  try {
    const ver = await invoke<string>("check_ffmpeg");
    ffmpegMsg.value = ver;
  } catch (e) {
    ffmpegOk.value = false;
    ffmpegMsg.value = String(e);
  }
  // GPU 检测改为按需（ffmpeg -encoders 很慢，不阻塞页面加载）
});

async function detectGpu() {
  if (gpuChecked.value) return;
  gpuChecked.value = true;
  try {
    const encoders = await invoke<any[]>("detect_gpu_encoders");
    gpuEncoders.value = encoders.map((e) => `${e.name} (${e.codec})`);
  } catch {}
}

// ====== 剪切 ======
const cutFile = ref("");
const cutStart = ref("00:00:00");
const cutEnd = ref("00:00:10");
const cutOutput = ref("");
const cutMode = ref("fast");

async function selectCutFile() {
  const selected = await open({
    filters: [{ name: "Video", extensions: ["mp4", "mov", "mkv", "webm", "avi"] }],
  });
  if (selected) cutFile.value = typeof selected === "string" ? selected : selected[0];
}

async function cutVideo() {
  if (!cutFile.value || !cutOutput.value) return;
  try {
    const msg = await invoke<string>("cut_video", {
      app: null,
      input: cutFile.value,
      start: cutStart.value,
      end: cutEnd.value,
      output: cutOutput.value,
      mode: cutMode.value,
    });
    ElMessage.success(msg);
  } catch (e) {
    ElMessage.error(String(e));
  }
}

// ====== 转码 ======
const transcodeFile = ref("");
const transcodeFormat = ref("h264");
const transcodeQuality = ref(23);
const transcodeOutput = ref("");

async function selectTranscodeFile() {
  const selected = await open({
    filters: [{ name: "Video", extensions: ["mp4", "mov", "mkv", "webm", "avi"] }],
  });
  if (selected) transcodeFile.value = typeof selected === "string" ? selected : selected[0];
}

async function transcodeVideo() {
  if (!transcodeFile.value || !transcodeOutput.value) return;
  try {
    const msg = await invoke<string>("transcode_video", {
      app: null,
      input: transcodeFile.value,
      outputFormat: transcodeFormat.value,
      videoCodec: transcodeFormat.value,
      crf: transcodeQuality.value,
      encoder: null,
      output: transcodeOutput.value,
    });
    ElMessage.success(msg);
  } catch (e) {
    ElMessage.error(String(e));
  }
}

// ====== GIF ======
const gifFile = ref("");
const gifStart = ref("00:00:00");
const gifDuration = ref(5);
const gifFps = ref(10);
const gifWidth = ref(480);
const gifOutput = ref("");

async function selectGifFile() {
  const selected = await open({
    filters: [{ name: "Video", extensions: ["mp4", "mov", "mkv", "webm", "avi"] }],
  });
  if (selected) gifFile.value = typeof selected === "string" ? selected : selected[0];
}

async function videoToGif() {
  if (!gifFile.value || !gifOutput.value) return;
  try {
    const msg = await invoke<string>("video_to_gif", {
      app: null,
      input: gifFile.value,
      start: gifStart.value,
      duration: gifDuration.value,
      fps: gifFps.value,
      width: gifWidth.value,
      output: gifOutput.value,
    });
    ElMessage.success(msg);
  } catch (e) {
    ElMessage.error(String(e));
  }
}
</script>

<template>
  <div class="page-container">
    <h2>{{ t("video.title") }}</h2>
    <p class="page-desc">{{ t("video.desc") }}</p>
    <el-alert
      v-if="!ffmpegOk"
      :title="ffmpegMsg || t('video.ffmpegMissing')"
      type="error"
      show-icon
      :closable="false"
      style="margin-bottom: 16px"
    >
      <template #default>
        <a href="https://ffmpeg.org/download.html" target="_blank" style="color: var(--el-color-primary)">
          {{ t("video.ffmpegDownload") }}
        </a>
        {{ t("video.ffmpegHint") }}
      </template>
    </el-alert>
    <div v-if="ffmpegOk && ffmpegMsg" class="ffmpeg-info">
      <el-tag type="success" size="small">{{ ffmpegMsg }}</el-tag>
      <el-button v-if="!gpuChecked" size="small" text type="primary" @click="detectGpu" style="margin-left:8px">
        检测 GPU 编码器
      </el-button>
    </div>
    <div v-if="gpuEncoders.length > 0" class="gpu-info">
      <el-tag v-for="enc in gpuEncoders" :key="enc" type="success" size="small">{{ enc }}</el-tag>
    </div>

    <el-tabs v-model="activeTab">
      <el-tab-pane :label="t('video.cut')" name="cut">
        <section class="section">
          <div class="drop-zone" @click="selectCutFile">
            <p>{{ cutFile ? cutFile.split(/[/\\]/).pop() : t("video.selectVideo") }}</p>
          </div>
          <el-form label-width="100px" class="form-section">
            <el-form-item :label="t('video.startTime')">
              <el-input v-model="cutStart" placeholder="00:00:00" />
            </el-form-item>
            <el-form-item :label="t('video.endTime')">
              <el-input v-model="cutEnd" placeholder="00:00:10" />
            </el-form-item>
            <el-form-item :label="t('video.mode')">
              <el-radio-group v-model="cutMode">
                <el-radio value="fast">{{ t("video.fast") }}</el-radio>
                <el-radio value="accurate">{{ t("video.accurate") }}</el-radio>
              </el-radio-group>
            </el-form-item>
            <el-form-item :label="t('pdf.outputPath')">
              <el-input v-model="cutOutput" placeholder="例如: D:/cut.mp4" />
            </el-form-item>
          </el-form>
          <el-button type="primary" :disabled="!cutFile || !cutOutput" @click="cutVideo">
            {{ t("video.cutVideo") }}
          </el-button>
        </section>
      </el-tab-pane>

      <el-tab-pane :label="t('video.transcode')" name="transcode">
        <section class="section">
          <div class="drop-zone" @click="selectTranscodeFile">
            <p>{{ transcodeFile ? transcodeFile.split(/[/\\]/).pop() : t("video.selectVideo") }}</p>
          </div>
          <el-form label-width="100px" class="form-section">
            <el-form-item :label="t('video.outputFormat')">
              <el-select v-model="transcodeFormat">
                <el-option label="H.264" value="h264" />
                <el-option label="H.265/HEVC" value="h265" />
                <el-option label="VP9" value="vp9" />
              </el-select>
            </el-form-item>
            <el-form-item :label="t('video.qualityCRF')">
              <el-slider v-model="transcodeQuality" :min="0" :max="51" show-input style="width: 300px" />
              <div class="form-hint">{{ t("video.qualityHint") }}</div>
            </el-form-item>
            <el-form-item :label="t('pdf.outputPath')">
              <el-input v-model="transcodeOutput" placeholder="例如: D:/output.mp4" />
            </el-form-item>
          </el-form>
          <el-button type="primary" :disabled="!transcodeFile || !transcodeOutput" @click="transcodeVideo">
            {{ t("video.transcodeVideo") }}
          </el-button>
        </section>
      </el-tab-pane>

      <el-tab-pane :label="t('video.gif')" name="gif">
        <section class="section">
          <div class="drop-zone" @click="selectGifFile">
            <p>{{ gifFile ? gifFile.split(/[/\\]/).pop() : t("video.selectVideo") }}</p>
          </div>
          <el-form label-width="100px" class="form-section">
            <el-form-item :label="t('video.startTime')">
              <el-input v-model="gifStart" placeholder="00:00:00" />
            </el-form-item>
            <el-form-item :label="t('video.duration')">
              <el-input-number v-model="gifDuration" :min="1" :max="30" />
            </el-form-item>
            <el-form-item :label="t('video.fps')">
              <el-input-number v-model="gifFps" :min="5" :max="30" />
            </el-form-item>
            <el-form-item :label="t('video.width')">
              <el-input-number v-model="gifWidth" :min="100" :max="1920" :step="10" />
            </el-form-item>
            <el-form-item :label="t('pdf.outputPath')">
              <el-input v-model="gifOutput" placeholder="例如: D:/output.gif" />
            </el-form-item>
          </el-form>
          <el-button type="primary" :disabled="!gifFile || !gifOutput" @click="videoToGif">
            {{ t("video.generateGif") }}
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
  margin: 0 0 8px;
}
.gpu-info {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 16px;
}
.ffmpeg-info {
  margin-bottom: 16px;
}
.section {
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
  padding: 16px 20px;
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
  margin-top: 4px;
}
</style>
