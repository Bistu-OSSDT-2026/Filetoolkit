<script setup lang="ts">
import { ref, computed } from "vue";
import { UploadFilled } from "@element-plus/icons-vue";

const props = withDefaults(
  defineProps<{
    accept?: string;
    multiple?: boolean;
  }>(),
  { accept: "*", multiple: true },
);

const emit = defineEmits<{
  (e: "files-selected", files: File[]): void;
}>();

const isDragging = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);
const acceptAttr = computed(() => props.accept === "*" ? undefined : props.accept);
let dragCounter = 0;

function onDragEnter(e: DragEvent) { e.preventDefault(); dragCounter++; isDragging.value = true; }
function onDragLeave(e: DragEvent) { e.preventDefault(); dragCounter--; if (dragCounter <= 0) { dragCounter = 0; isDragging.value = false; } }
function onDragOver(e: DragEvent) { e.preventDefault(); }

function onDrop(e: DragEvent) {
  e.preventDefault(); dragCounter = 0; isDragging.value = false;
  const files = e.dataTransfer?.files;
  if (!files || files.length === 0) return;
  emit("files-selected", Array.from(files));
}

function onClick() { fileInput.value?.click(); }

function onInputChange(e: Event) {
  const input = e.target as HTMLInputElement;
  const files = input.files;
  if (!files || files.length === 0) return;
  emit("files-selected", Array.from(files));
  input.value = "";
}
</script>

<template>
  <div class="file-drop-zone" :class="{ dragging: isDragging }"
    @dragenter="onDragEnter" @dragleave="onDragLeave" @dragover="onDragOver" @drop="onDrop" @click="onClick">
    <input ref="fileInput" type="file" :accept="acceptAttr" :multiple="multiple" class="file-input-hidden" @change="onInputChange" />
    <div class="drop-content">
      <el-icon class="drop-icon" :size="40"><UploadFilled /></el-icon>
      <p class="drop-text">{{ isDragging ? "松开以添加文件" : "拖拽文件到此处,或点击选择" }}</p>
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
  transition: border-color 0.3s, background-color 0.3s;
  user-select: none;
}
.file-drop-zone:hover, .file-drop-zone.dragging {
  border-color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-9);
}
.file-input-hidden { display: none; }
.drop-content { display: flex; flex-direction: column; align-items: center; gap: 8px; }
.drop-icon { color: var(--el-text-color-placeholder); }
.drop-text { margin: 0; font-size: 14px; color: var(--el-text-color-regular); }
.drop-hint { margin: 0; font-size: 12px; color: var(--el-text-color-secondary); }
</style>
