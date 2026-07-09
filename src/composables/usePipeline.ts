import { ref, type Ref } from "vue";
import type { Node, Edge, Connection } from "@vue-flow/core";
import type { Pipeline, NodeType, ParamDef } from "../pipeline/types";

/** 硬编码的节点类型注册表（前期不与后端通信，后续可改为 invoke("get_node_types")） */
const BUILTIN_NODE_TYPES: NodeType[] = [
  {
    id: "image_compress",
    name: "图片压缩",
    category: "图片",
    description: "批量压缩图片并转换格式，支持调整尺寸",
    icon: "Picture",
    inputs: [{ id: "input", label: "图片文件", portType: "file[]" }],
    outputs: [{ id: "output", label: "压缩后图片", portType: "file[]" }],
    params: [
      {
        key: "quality",
        label: "质量",
        paramType: { slider: { min: 1, max: 100, step: 1 } },
        default: 80,
        help: "数值越大质量越高",
      },
      {
        key: "format",
        label: "输出格式",
        paramType: "select",
        default: "webp",
        options: [
          { value: "jpg", label: "JPG" },
          { value: "png", label: "PNG" },
          { value: "webp", label: "WebP" },
          { value: "avif", label: "AVIF" },
        ],
      },
      { key: "maxWidth", label: "最大宽度(px)", paramType: "number", help: "超过此宽度会等比缩放" },
      {
        key: "maxHeight",
        label: "最大高度(px)",
        paramType: "number",
        help: "超过此高度会等比缩放",
      },
    ],
  },
  {
    id: "pdf_merge",
    name: "PDF 合并",
    category: "文档",
    description: "将多个 PDF 合并为一个文件",
    icon: "Document",
    inputs: [{ id: "input", label: "PDF 文件(多个)", portType: "file[]" }],
    outputs: [{ id: "output", label: "合并后 PDF", portType: "file" }],
    params: [
      { key: "outputName", label: "输出文件名", paramType: "string", default: "merged.pdf" },
    ],
  },
  {
    id: "pdf_split",
    name: "PDF 拆分",
    category: "文档",
    description: "按页码范围将 PDF 拆分为多个文件",
    icon: "Document",
    inputs: [{ id: "input", label: "PDF 文件", portType: "file" }],
    outputs: [{ id: "output", label: "拆分后 PDF", portType: "file[]" }],
    params: [
      {
        key: "ranges",
        label: "页码范围",
        paramType: "string",
        default: "1-5",
        required: true,
        help: "每行一个范围，如 1-5",
      },
    ],
  },
  {
    id: "pdf_compress",
    name: "PDF 压缩",
    category: "文档",
    description: "压缩 PDF 文件大小",
    icon: "Document",
    inputs: [{ id: "input", label: "PDF 文件", portType: "file" }],
    outputs: [{ id: "output", label: "压缩后 PDF", portType: "file" }],
    params: [],
  },
  {
    id: "file_rename",
    name: "批量重命名",
    category: "文件",
    description: "使用模板变量批量重命名文件",
    icon: "EditPen",
    inputs: [{ id: "input", label: "文件", portType: "file[]" }],
    outputs: [{ id: "output", label: "重命名后文件", portType: "file[]" }],
    params: [
      {
        key: "pattern",
        label: "命名模板",
        paramType: "string",
        default: "{name}-{index:3}",
        required: true,
        help: "支持 {name} {ext} {date} {index:3} 等变量",
      },
    ],
  },
  {
    id: "file_dedup",
    name: "重复查重",
    category: "文件",
    description: "按哈希值查找并删除重复文件",
    icon: "Search",
    inputs: [{ id: "input", label: "文件", portType: "file[]" }],
    outputs: [{ id: "output", label: "去重后文件", portType: "file[]" }],
    params: [
      {
        key: "keepStrategy",
        label: "保留策略",
        paramType: "select",
        default: "newest",
        options: [
          { value: "newest", label: "保留最新" },
          { value: "largest", label: "保留最大" },
          { value: "first", label: "保留第一个" },
        ],
      },
    ],
  },
  {
    id: "file_archive",
    name: "文件打包",
    category: "文件",
    description: "将文件打包为 zip/7z/tar.gz",
    icon: "FolderOpened",
    inputs: [{ id: "input", label: "文件/目录", portType: "file[]" }],
    outputs: [{ id: "output", label: "压缩包", portType: "file" }],
    params: [
      {
        key: "format",
        label: "压缩格式",
        paramType: "select",
        default: "zip",
        options: [
          { value: "zip", label: "ZIP" },
          { value: "7z", label: "7Z" },
          { value: "tar.gz", label: "TAR.GZ" },
        ],
      },
      {
        key: "archiveName",
        label: "压缩包名称",
        paramType: "string",
        default: "archive",
      },
    ],
  },
  {
    id: "video_transcode",
    name: "视频转码",
    category: "视频",
    description: "转换视频格式和编码",
    icon: "VideoCamera",
    inputs: [{ id: "input", label: "视频文件", portType: "file[]" }],
    outputs: [{ id: "output", label: "转码后视频", portType: "file[]" }],
    params: [
      {
        key: "format",
        label: "输出格式",
        paramType: "select",
        default: "mp4",
        options: [
          { value: "mp4", label: "MP4" },
          { value: "mov", label: "MOV" },
          { value: "mkv", label: "MKV" },
          { value: "webm", label: "WebM" },
        ],
      },
      {
        key: "codec",
        label: "编码",
        paramType: "select",
        default: "h264",
        options: [
          { value: "h264", label: "H.264" },
          { value: "h265", label: "H.265" },
        ],
      },
    ],
  },
  {
    id: "video_cut",
    name: "视频剪切",
    category: "视频",
    description: "按时间范围裁剪视频片段",
    icon: "VideoCamera",
    inputs: [{ id: "input", label: "视频文件", portType: "file" }],
    outputs: [{ id: "output", label: "剪切后视频", portType: "file" }],
    params: [
      {
        key: "start",
        label: "开始时间",
        paramType: "string",
        default: "00:00:00",
        required: true,
        help: "格式 HH:MM:SS",
      },
      {
        key: "end",
        label: "结束时间",
        paramType: "string",
        default: "00:01:00",
        required: true,
        help: "格式 HH:MM:SS",
      },
    ],
  },
];

export function groupedNodeTypes(): Map<string, NodeType[]> {
  const map = new Map<string, NodeType[]>();
  for (const nt of BUILTIN_NODE_TYPES) {
    const list = map.get(nt.category) ?? [];
    list.push(nt);
    map.set(nt.category, list);
  }
  return map;
}

export function findNodeType(id: string): NodeType | undefined {
  return BUILTIN_NODE_TYPES.find((nt) => nt.id === id);
}

/**
 * 流水线画布状态管理。
 * 接收 Vue Flow 的 nodes/edges refs，避免重复创建导致类型冲突。
 */
export class PipelineState {
  pipelineName = ref("未命名流水线");
  pipelineDescription = ref("");
  selectedNodeId = ref<string | null>(null);
  validationErrors = ref<string[]>([]);

  private nodesRef: Ref<Node[]>;
  private edgesRef: Ref<Edge[]>;

  constructor(nodesRef: Ref<Node[]>, edgesRef: Ref<Edge[]>) {
    this.nodesRef = nodesRef;
    this.edgesRef = edgesRef;
  }

  get nodes(): Node[] {
    return this.nodesRef.value;
  }

  get edges(): Edge[] {
    return this.edgesRef.value;
  }

  getSelectedNode(): Node | null {
    if (!this.selectedNodeId.value) return null;
    return this.nodes.find((n) => n.id === this.selectedNodeId.value) ?? null;
  }

  getSelectedNodeType(): NodeType | null {
    const node = this.getSelectedNode();
    if (!node) return null;
    return findNodeType(String(node.type ?? "")) ?? null;
  }

  getSelectedNodeParams(): Record<string, unknown> {
    const node = this.getSelectedNode();
    return (node?.data?.params as Record<string, unknown>) ?? {};
  }

  // ========== 节点操作 ==========

  addNode(nodeTypeId: string, position: { x: number; y: number }) {
    const nt = findNodeType(nodeTypeId);
    if (!nt) return;
    const id = `node-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    this.nodesRef.value.push({
      id,
      type: nodeTypeId,
      position,
      label: nt.name,
      data: { nodeType: nodeTypeId, label: nt.name, params: buildDefaultParams(nt.params ?? []) },
    });
    this.selectedNodeId.value = id;
  }

  removeNode(nodeId: string) {
    this.nodesRef.value = this.nodesRef.value.filter((n) => n.id !== nodeId);
    this.edgesRef.value = this.edgesRef.value.filter(
      (e) => e.source !== nodeId && e.target !== nodeId,
    );
    if (this.selectedNodeId.value === nodeId) this.selectedNodeId.value = null;
  }

  updateNodeParam(nodeId: string, key: string, value: unknown) {
    const node = this.nodesRef.value.find((n) => n.id === nodeId);
    if (node?.data) {
      node.data = { ...node.data, params: { ...node.data.params, [key]: value } };
    }
  }

  // ========== 连线操作 ==========

  addEdge(connection: Connection) {
    if (!connection.source || !connection.target) return;
    const id = `edge-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    this.edgesRef.value.push({
      id,
      source: connection.source,
      target: connection.target,
      sourceHandle: connection.sourceHandle ?? undefined,
      targetHandle: connection.targetHandle ?? undefined,
    });
  }

  removeEdge(edgeId: string) {
    this.edgesRef.value = this.edgesRef.value.filter((e) => e.id !== edgeId);
  }

  // ========== 校验 ==========

  validate(): string[] {
    const errors: string[] = [];
    if (!this.isDAG()) errors.push("流水线存在循环依赖，请检查连线");
    for (const node of this.nodes) {
      const nt = findNodeType(String(node.type ?? ""));
      if (!nt) continue;
      const nodeParams = (node.data?.params ?? {}) as Record<string, unknown>;
      for (const p of (nt.params ?? []).filter((p) => p.required)) {
        if (!nodeParams[p.key] && nodeParams[p.key] !== 0 && nodeParams[p.key] !== false) {
          errors.push(`节点「${node.label ?? node.id}」缺少必填参数: ${p.label}`);
        }
      }
    }
    if (this.nodes.length > 1) {
      const connected = new Set(this.edges.flatMap((e) => [e.source, e.target]));
      const orphans = this.nodes.filter((n) => !connected.has(n.id));
      if (orphans.length > 0 && orphans.length < this.nodes.length) {
        errors.push(`${orphans.length} 个节点未连接到流水线中`);
      }
    }
    this.validationErrors.value = errors;
    return errors;
  }

  private isDAG(): boolean {
    const nodeIds = new Set(this.nodes.map((n) => n.id));
    const inDegree = new Map<string, number>();
    const adj = new Map<string, string[]>();
    for (const id of nodeIds) {
      inDegree.set(id, 0);
      adj.set(id, []);
    }
    for (const e of this.edges) {
      if (!nodeIds.has(e.source) || !nodeIds.has(e.target)) continue;
      adj.get(e.source)!.push(e.target);
      inDegree.set(e.target, (inDegree.get(e.target) ?? 0) + 1);
    }
    const queue = [...inDegree.entries()].filter(([, d]) => d === 0).map(([id]) => id);
    let visited = 0;
    while (queue.length > 0) {
      const curr = queue.shift()!;
      visited++;
      for (const next of adj.get(curr) ?? []) {
        const d = (inDegree.get(next) ?? 1) - 1;
        inDegree.set(next, d);
        if (d === 0) queue.push(next);
      }
    }
    return visited === nodeIds.size;
  }

  // ========== 序列化 ==========

  toPipelineJSON(): Pipeline {
    return {
      name: this.pipelineName.value,
      description: this.pipelineDescription.value || undefined,
      nodes: this.nodes.map((n) => ({
        id: n.id,
        nodeType: String(n.type ?? ""),
        label: String(n.label ?? n.data?.label ?? ""),
        position: { x: n.position.x, y: n.position.y },
        params: (n.data?.params ?? {}) as Record<string, unknown>,
      })),
      edges: this.edges.map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourcePort: e.sourceHandle ?? undefined,
        targetPort: e.targetHandle ?? undefined,
      })),
    };
  }

  fromPipelineJSON(pipeline: Pipeline) {
    this.pipelineName.value = pipeline.name;
    this.pipelineDescription.value = pipeline.description ?? "";
    this.nodesRef.value = pipeline.nodes.map((n) => ({
      id: n.id,
      type: n.nodeType,
      position: n.position,
      label: n.label,
      data: { nodeType: n.nodeType, label: n.label, params: n.params ?? {} },
    }));
    this.edgesRef.value = pipeline.edges.map((e) => ({
      id: e.id,
      source: e.source,
      target: e.target,
      sourceHandle: e.sourcePort ?? undefined,
      targetHandle: e.targetPort ?? undefined,
    }));
  }

  clearCanvas() {
    this.nodesRef.value = [];
    this.edgesRef.value = [];
    this.selectedNodeId.value = null;
    this.validationErrors.value = [];
    this.pipelineName.value = "未命名流水线";
    this.pipelineDescription.value = "";
  }
}

function buildDefaultParams(paramDefs: ParamDef[]): Record<string, unknown> {
  const params: Record<string, unknown> = {};
  for (const p of paramDefs) {
    if (p.default !== undefined) params[p.key] = p.default;
  }
  return params;
}
