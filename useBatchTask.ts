import { ref, onUnmounted, readonly } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useTaskStore } from "../store/task";
import type { TaskStatus } from "../types/task";

/**
 * 批量任务执行抽象。
 * 封装了 "invoke 后端命令 → 监听 task-progress 事件 → 更新 Pinia store" 的标准流程。
 *
 * 用法:
 *   const { run, cancel, progress, status } = useBatchTask("compress_images");
 *   const results = await run("compress_images", { files, quality: 80 });
 */

export interface TaskProgress {
  progress: number;
  status: TaskStatus;
  message: string;
}

interface ProgressPayload {
  current: number;
  total: number;
  message?: string;
}

export function useBatchTask(taskName: string) {
  const store = useTaskStore();

  const progress = ref(0);
  const status = ref<TaskStatus>("idle");
  const message = ref("");

  let unlisten: UnlistenFn | null = null;

  function generateTaskId(): string {
    return `${taskName}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  }

  async function run<T>(invokeCmd: string, args: Record<string, unknown> = {}): Promise<T> {
    const taskId = generateTaskId();

    progress.value = 0;
    status.value = "running";
    message.value = "";
    store.startTask(taskId, taskName);

    try {
      unlisten = await listen<ProgressPayload>("task-progress", (event) => {
        const { current, total } = event.payload;
        const msg = event.payload.message ?? "";
        progress.value = total > 0 ? Math.round((current / total) * 100) : 0;
        message.value = msg;
        store.updateProgress(current, total, msg);
      });
    } catch {
      // 非 Tauri 环境时忽略
    }

    try {
      const result = await invoke<T>(invokeCmd, args);

      progress.value = 100;
      status.value = "done";
      store.completeTask(taskId);

      return result;
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);

      if (errMsg.includes("cancelled") || errMsg.includes("取消")) {
        status.value = "cancelled";
        message.value = "任务已取消";
        store.cancelTask();
      } else {
        status.value = "error";
        message.value = errMsg;
        store.failTask(taskId, errMsg);
      }

      throw err;
    } finally {
      cleanup();
    }
  }

  function cancel() {
    if (status.value === "running") {
      invoke("cancel_batch").catch(() => {});
      store.cancelTask();
      status.value = "cancelled";
      message.value = "正在取消...";
    }
  }

  function cleanup() {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
  }

  onUnmounted(cleanup);

  return {
    run,
    cancel,
    progress: readonly(progress),
    status: readonly(status),
    message: readonly(message),
  };
}
