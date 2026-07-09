import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

/** 目录树节点，与 Rust 端 DirNode 一一对应 */
export interface DirNode {
  name: string;
  path: string;
  size: number;
  isDir: boolean;
  children?: DirNode[];
}

/**
 * 磁盘扫描 composable。
 * 调用后端 scan_directory 命令，返回 DirNode 树。
 */
export function useDiskScan() {
  const scanning = ref(false);
  const progress = ref("");
  const data = ref<DirNode | null>(null);
  const error = ref("");

  async function scan(dir: string) {
    scanning.value = true;
    progress.value = "正在扫描...";
    error.value = "";
    data.value = null;

    try {
      const result = await invoke<DirNode>("scan_directory", { dir });
      data.value = result;
      progress.value = `扫描完成: ${formatBytes(result.size)}`;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      scanning.value = false;
    }
  }

  return { scanning, progress, data, error, scan };
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const size = bytes / Math.pow(1024, i);
  return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
