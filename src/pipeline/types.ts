// ============================================================
// pipeline/types.ts —— 流水线前端类型定义
//
// 此文件与 Rust 端 src-tauri/src/pipeline/model.rs 的类型一一对应。
// 前端编辑器构建的所有 JSON 必须符合这些类型。
// 修改时务必与后端同步！！！
//
// 约定人: A(后端) → D(前端)
// 日期: 2026-07-07
// ============================================================

// ============ 顶层 ============

/** 一条流水线 */
export interface Pipeline {
  name: string;
  description?: string;
  nodes: PipelineNode[];
  edges: PipelineEdge[];
}

// ============ 节点 ============

/** 流水线中的一个操作节点 */
export interface PipelineNode {
  /** 节点唯一 ID,如 "node-1" */
  id: string;
  /** 节点类型 ID,对应 NodeTypeRegistry 中的 key,如 "image_compress" */
  nodeType: string;
  /** 显示标签 */
  label: string;
  /** 画布坐标(前端用,后端忽略) */
  position: NodePosition;
  /** 节点参数,结构由对应 NodeType 的 inputSchema 定义 */
  params?: Record<string, unknown>;
}

export interface NodePosition {
  x: number;
  y: number;
}

// ============ 连线 ============

/** 一条连线:source 的输出 → target 的输入 */
export interface PipelineEdge {
  id: string;
  /** 源节点 ID */
  source: string;
  /** 源端口(默认 "default") */
  sourcePort?: string;
  /** 目标节点 ID */
  target: string;
  /** 目标端口(默认 "default") */
  targetPort?: string;
}

// ============ 节点类型注册(编辑器面板用) ============

/** 节点类型定义 —— 编辑器根据此信息渲染面板和参数表单 */
export interface NodeType {
  /** 类型标识 */
  id: string;
  /** 显示名称 */
  name: string;
  /** 分类(前端按分类分组: "图片" / "文档" / "文件" / "视频" / "音频") */
  category: string;
  /** 描述 */
  description: string;
  /** Element Plus 图标名,如 "Picture" / "Document" */
  icon?: string;
  /** 输入端口 */
  inputs?: PortDef[];
  /** 输出端口 */
  outputs?: PortDef[];
  /** 参数定义列表 */
  params?: ParamDef[];
}

export interface PortDef {
  id: string;
  label: string;
  /** 端口类型: "file" | "file[]" | "string" | "number" */
  portType?: string;
}

// ============ 参数定义 ============

export interface ParamDef {
  /** 参数 key,如 "quality" */
  key: string;
  /** 显示标签 */
  label: string;
  /** 参数类型 */
  paramType: ParamType;
  /** 默认值 */
  default?: unknown;
  /** 是否必填 */
  required?: boolean;
  /** 帮助提示 */
  help?: string;
  /** 选项(paramType 为 "select" 时) */
  options?: SelectOption[];
}

/** 参数类型 */
export type ParamType =
  | "string"
  | "number"
  | "boolean"
  | "select"
  | "file"
  | "directory"
  | { slider: { min: number; max: number; step: number } };

export interface SelectOption {
  value: string;
  label: string;
}

// ============ 运行时状态(后端 → 前端事件) ============

export interface PipelineProgress {
  pipelineId: string;
  currentStep: number;
  totalSteps: number;
  currentNodeId?: string;
  currentNodeLabel?: string;
  status: PipelineStatus;
}

export type PipelineStatus =
  "ready" | "running" | "paused" | "completed" | { failed: string } | "cancelled";
