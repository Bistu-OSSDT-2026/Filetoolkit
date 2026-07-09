<template>
  <div style="padding: 40px; max-width: 900px;">
    <h1>批量文件重命名工具</h1>
    <div style="margin: 20px 0;">
      <p>文件夹路径：</p >
      <input
        v-model="folderPath"
        style="width: 100%; padding: 8px; font-size: 16px;"
        placeholder="粘贴需要处理的文件夹完整路径"
      />
    </div>
    <div style="margin: 20px 0;">
      <p>统一文件前缀：</p >
      <input
        v-model="filePrefix"
        style="width: 100%; padding: 8px; font-size: 16px;"
        placeholder="例如：photo、document"
      />
    </div>
    <button
      @click="runRename"
      style="padding: 10px 30px; font-size: 16px; background: #2478ff; color: white; border: none; border-radius: 6px; cursor: pointer;"
    >
      开始批量重命名
    </button>
    <div style="margin-top: 30px; font-size: 16px;">
      {{ tipText }}
    </div>
  </div>
</template>

<script setup>
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const folderPath = ref("");
const filePrefix = ref("");
const tipText = ref("");

const runRename = async () => {
  tipText.value = "正在处理文件，请稍等...";
  try {
    await invoke("rename_files", {
      dir: folderPath.value,
      pre: filePrefix.value,
    });
    tipText.value = "✅ 所有文件重命名完成！";
  } catch (err) {
    tipText.value = "❌ 执行失败：" + err;
  }
};
</script>