import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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
 * 调用后端 scan_directory 命令 + 监听 disk-scan-progress 事件。
 */
export function useDiskScan() {
  const scanning = ref(false);
  const progress = ref("");
  const data = ref<DirNode | null>(null);
  const error = ref("");
  let unlisten: UnlistenFn | null = null;

  async function scan(dir: string) {
    scanning.value = true;
    progress.value = "正在扫描...";
    error.value = "";
    data.value = null;

    // 监听进度事件
    try {
      unlisten = await listen<{ message: string }>("disk-scan-progress", (event) => {
        progress.value = event.payload.message;
      });
    } catch {
      // 非 Tauri 环境忽略
    }

    try {
      const result = await invoke<DirNode>("scan_directory", { dir });
      data.value = result;
      progress.value = `扫描完成: ${formatBytes(result.size)}`;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      scanning.value = false;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    }
  }

  return { scanning, progress, data, error, scan };
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
