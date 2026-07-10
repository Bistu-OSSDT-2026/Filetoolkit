<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpened, RefreshRight } from "@element-plus/icons-vue";
import SunburstChart from "../components/SunburstChart.vue";
import { useDiskScan } from "../composables/useDiskScan";

const { t } = useI18n();

const { scanning, progress, data, error, scan } = useDiskScan();

const selectedDir = ref("");

/** Tauri 原生文件夹选择 → 自动扫描 */
async function selectAndScan() {
  const dir = await open({ directory: true });
  if (!dir) return;
  const path = typeof dir === "string" ? dir : dir[0];
  selectedDir.value = path;
  await scan(path);
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
</script>

<template>
  <div class="disk-usage-page">
    <div class="du-header">
      <h2>
        <el-icon :size="24"><FolderOpened /></el-icon>
        {{ t("diskUsage.title") }}
      </h2>
      <div class="du-controls">
        <el-input v-model="selectedDir" :placeholder="t('diskUsage.directoryPath')" size="default" style="width: 360px" clearable />
        <el-button type="primary" :loading="scanning" @click="selectAndScan">
          <el-icon><FolderOpened /></el-icon>{{ t("diskUsage.selectDir") }}
        </el-button>
        <el-button v-if="selectedDir" :loading="scanning" @click="scan(selectedDir)">
          <el-icon><RefreshRight /></el-icon>{{ t("diskUsage.rescan") }}
        </el-button>
      </div>
    </div>

    <!-- 扫描状态 -->
    <div v-if="scanning || progress" class="du-status">
      <el-alert v-if="scanning" :title="progress" type="info" :closable="false" show-icon />
      <el-alert v-else-if="error" :title="error" type="error" :closable="false" show-icon />
      <el-alert v-else-if="data" type="success" :closable="false" show-icon>
        <template #title>
          扫描完成：
          <b>{{ data.name }}</b>
          &mdash; 总大小 <b>{{ formatBytes(data.size) }}</b>
        </template>
      </el-alert>
    </div>

    <!-- 旭日图 -->
    <div class="du-chart">
      <SunburstChart :data="data" />
    </div>

    <div v-if="!data && !scanning" class="du-empty">
      <el-icon :size="48" color="var(--el-text-color-placeholder)">
        <FolderOpened />
      </el-icon>
      <p>选择目录，查看磁盘占用分布</p>
      <p class="du-hint">支持 10GB+ 大目录，秒级扫描</p>
    </div>
  </div>
</template>

<style scoped>
.disk-usage-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 20px 24px;
}

.du-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
  flex-shrink: 0;
}

.du-header h2 {
  margin: 0;
  font-size: 1.2em;
  display: flex;
  align-items: center;
  gap: 8px;
}

.du-controls {
  display: flex;
  gap: 8px;
}

.du-status {
  margin-bottom: 12px;
  flex-shrink: 0;
}

.du-chart {
  flex: 1;
  min-height: 0;
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
  background-color: var(--el-bg-color);
  overflow: hidden;
}

.du-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--el-text-color-secondary);
  gap: 4px;
}

.du-hint {
  font-size: 0.85em;
  color: var(--el-text-color-placeholder);
}
</style>
