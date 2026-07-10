import { ref } from "vue";
import type { Pipeline } from "../pipeline/types";
import photoOrganize from "../assets/templates/photo-organize.json";
import contractStandardize from "../assets/templates/contract-standardize.json";

const STORAGE_KEY = "filetoolkit:pipeline-templates";

export interface TemplateEntry {
  /** 唯一 ID */
  id: string;
  /** 模板名称 */
  name: string;
  /** 描述 */
  description?: string;
  /** 来源: builtin 或 user */
  source: "builtin" | "user";
  /** 模板数据 */
  pipeline: Pipeline;
  /** 创建时间 */
  createdAt: number;
}

/** 内置模板 */
const BUILTIN_TEMPLATES: TemplateEntry[] = [
  {
    id: "builtin-photo-organize",
    name: "照片整理归档",
    description: "图片压缩(WebP,1920px) → 重命名 → 打包(zip)",
    source: "builtin",
    pipeline: photoOrganize as Pipeline,
    createdAt: 0,
  },
  {
    id: "builtin-contract-standardize",
    name: "合同 PDF 标准化",
    description: "PDF 合并 → 压缩 → 加密",
    source: "builtin",
    pipeline: contractStandardize as Pipeline,
    createdAt: 0,
  },
];

/**
 * 模板管理 composable。
 * 维护内置模板 + 用户自定义模板，持久化到 localStorage。
 */
export function useTemplates() {
  const templates = ref<TemplateEntry[]>(loadAll());
  const selectedTemplateId = ref<string | null>(null);

  /** 加载所有模板（内置 + localStorage 中的用户模板） */
  function loadAll(): TemplateEntry[] {
    const userTemplates = loadUserTemplates();
    return [...BUILTIN_TEMPLATES, ...userTemplates];
  }

  /** 从 localStorage 加载用户保存的模板 */
  function loadUserTemplates(): TemplateEntry[] {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return [];
      const parsed: TemplateEntry[] = JSON.parse(raw);
      return parsed.filter((t) => t.source === "user");
    } catch {
      return [];
    }
  }

  /** 保存用户模板到 localStorage */
  function saveUserTemplates(userTemplates: TemplateEntry[]) {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(userTemplates));
    } catch {
      // localStorage 不可用（私密模式等），静默忽略
    }
  }

  /** 将当前画布另存为模板 */
  function saveAsTemplate(pipeline: Pipeline): TemplateEntry {
    const entry: TemplateEntry = {
      id: `user-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      name: pipeline.name || "未命名模板",
      description: pipeline.description,
      source: "user",
      pipeline: JSON.parse(JSON.stringify(pipeline)), // 深拷贝
      createdAt: Date.now(),
    };
    const userTemplates = templates.value.filter((t) => t.source === "user");
    userTemplates.unshift(entry);
    saveUserTemplates(userTemplates);
    templates.value = loadAll();
    return entry;
  }

  /** 删除用户模板 */
  function deleteTemplate(id: string) {
    const entry = templates.value.find((t) => t.id === id);
    if (!entry || entry.source === "builtin") return;
    const userTemplates = templates.value.filter((t) => t.source === "user" && t.id !== id);
    saveUserTemplates(userTemplates);
    templates.value = loadAll();
    if (selectedTemplateId.value === id) selectedTemplateId.value = null;
  }

  /** 导出模板为 JSON 文件下载 */
  function exportTemplate(entry: TemplateEntry) {
    const blob = new Blob([JSON.stringify(entry.pipeline, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${entry.name || "template"}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  return {
    templates,
    selectedTemplateId,
    saveAsTemplate,
    deleteTemplate,
    exportTemplate,
    refresh: () => {
      templates.value = loadAll();
    },
  };
}
