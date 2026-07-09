<script setup lang="ts">
import { ref, computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { UploadFilled } from "@element-plus/icons-vue";

/**
 * 文件拖拽/选择通用组件。
 * Tauri 环境下使用原生对话框（返回真实文件路径），
 * 浏览器 dev 模式下回退到 HTML input（路径不可用）。
 *
 * Props:
 *   accept   — 限制可选文件类型(如 "image/*" / ".pdf")
 *   multiple — 是否允许多选(默认 true)
 *
 * Emit:
 *   @files-selected — 用户选择文件后发出真实文件路径 string[]
 */

const props = withDefaults(
  defineProps<{
    accept?: string;
    multiple?: boolean;
  }>(),
  {
    accept: "*",
    multiple: true,
  },
);

const emit = defineEmits<{
  (e: "files-selected", files: string[]): void;
}>();

const isDragging = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

let dragCounter = 0;

/** 检测是否在 Tauri 环境 */
const isTauri = !!(window as any).__TAURI_INTERNALS__;

/** MIME 类型 → 扩展名映射（Tauri 对话框只接受扩展名，不接受 MIME） */
const MIME_TO_EXT: Record<string, string[]> = {
  "image/*": ["jpg", "jpeg", "png", "gif", "webp", "bmp", "avif", "tiff"],
  "audio/*": ["mp3", "wav", "ogg", "flac", "aac", "m4a"],
  "video/*": ["mp4", "mov", "mkv", "webm", "avi"],
};

/** 将 accept 属性转为 Tauri 对话框 filters */
const tauriFilters = computed(() => {
  if (!props.accept || props.accept === "*") return [];
  return props.accept.split(",").map((s) => {
    const mime = s.trim();
    // MIME 类型组 → 展开为扩展名列表
    if (MIME_TO_EXT[mime]) {
      return {
        name: mime.replace("/*", "").toUpperCase(),
        extensions: MIME_TO_EXT[mime],
      };
    }
    // 扩展名：移除前导 * 和 .
    const ext = mime.replace(/^\*?\.?/, "").toLowerCase();
    return { name: ext.toUpperCase(), extensions: [ext] };
  });
});

/** 点击 → Tauri 原生对话框（路径） 或 浏览器 input */
async function onClick() {
  if (isTauri) {
    // Tauri：原生文件选择，返回真实路径
    const selected = await open({
      multiple: props.multiple,
      filters: tauriFilters.value.length > 0 ? tauriFilters.value : undefined,
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length > 0) emit("files-selected", paths);
  } else {
    // 浏览器回退
    fileInput.value?.click();
  }
}

/** HTML input 选中（仅浏览器环境） */
function onInputChange(e: Event) {
  const input = e.target as HTMLInputElement;
  const files = input.files;
  if (!files || files.length === 0) return;

  const paths: string[] = [];
  for (let i = 0; i < files.length; i++) {
    // 浏览器不支持 .path，但尝试读取
    const p = (files[i] as any).path;
    if (p && typeof p === "string" && (p.includes("\\") || p.includes("/"))) {
      paths.push(p);
    }
  }
  if (paths.length > 0) {
    emit("files-selected", paths);
  } else {
    // 浏览器环境无法获取路径
    alert("浏览器不支持直接读取文件路径，请在 Tauri 桌面应用中使用此功能。");
  }
  input.value = "";
}

// 拖拽保留（浏览器环境可能用上）
function onDragEnter(e: DragEvent) {
  e.preventDefault();
  dragCounter++;
  isDragging.value = true;
}

function onDragLeave(e: DragEvent) {
  e.preventDefault();
  dragCounter--;
  if (dragCounter <= 0) {
    dragCounter = 0;
    isDragging.value = false;
  }
}

function onDragOver(e: DragEvent) {
  e.preventDefault();
}

function onDrop(e: DragEvent) {
  e.preventDefault();
  dragCounter = 0;
  isDragging.value = false;
  // Tauri 环境下走原生对话框，拖放不支持路径获取
  if (isTauri) return;
  // 浏览器环境尝试
  const files = e.dataTransfer?.files;
  if (!files || files.length === 0) return;
  onInputChange({ target: { files, value: "" } } as any);
}
</script>

<template>
  <div
    class="file-drop-zone"
    :class="{ dragging: isDragging }"
    @dragenter="onDragEnter"
    @dragleave="onDragLeave"
    @dragover="onDragOver"
    @drop="onDrop"
    @click="onClick"
  >
    <input
      v-if="!isTauri"
      ref="fileInput"
      type="file"
      :accept="props.accept === '*' ? undefined : props.accept"
      :multiple="props.multiple"
      class="file-input-hidden"
      @change="onInputChange"
    />

    <div class="drop-content">
      <el-icon class="drop-icon" :size="40">
        <UploadFilled />
      </el-icon>
      <p class="drop-text">
        <template v-if="isDragging"> 松开以添加文件 </template>
        <template v-else> 点击选择文件 </template>
      </p>
      <p v-if="accept && accept !== '*'" class="drop-hint">支持格式: {{ accept }}</p>
    </div>
  </div>
</template>

<style scoped>
.file-drop-zone {
  border: 2px dashed var(--el-border-color);
  border-radius: 8px;
  padding: 40px 20px;
  text-align: center;
  cursor: pointer;
  transition:
    border-color 0.3s,
    background-color 0.3s;
  user-select: none;
}

.file-drop-zone:hover,
.file-drop-zone.dragging {
  border-color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-9);
}

.file-input-hidden {
  display: none;
}

.drop-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.drop-icon {
  color: var(--el-text-color-placeholder);
}

.drop-text {
  margin: 0;
  font-size: 14px;
  color: var(--el-text-color-regular);
}

.drop-hint {
  margin: 0;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
</style>
