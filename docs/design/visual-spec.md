# Visual Specification — FileToolkit 视觉规范

> 版本: v1.0 | 日期: 2026-07-10 | 负责人: 成员 D

## 1. 色彩系统

### 品牌色

| 变量 | 浅色 | 深色 | 用途 |
|------|------|------|------|
| `--ft-primary` | `#409eff` | `#409eff` | 主按钮、链接、选中态 |
| `--ft-primary-light` | `#66b1ff` | `#337ecc` | hover 态 |
| `--ft-primary-dark` | `#3a8ee6` | `#66b1ff` | active 态 |

### 中性色

| 变量 | 浅色 | 深色 | 用途 |
|------|------|------|------|
| `--ft-bg` | `#f5f7fa` | `#1a1a2e` | 页面背景 |
| `--ft-bg-card` | `#ffffff` | `#222244` | 卡片/面板背景 |
| `--ft-border` | `#e4e7ed` | `#333355` | 边框 |
| `--ft-sidebar-bg` | `#ffffff` | `#1e1e3a` | 侧边栏背景 |

### 状态色

沿用 Element Plus 默认语义色:
- 成功 `#67c23a` | 警告 `#e6a23c` | 危险 `#f56c6c` | 信息 `#909399`

## 2. 间距规范

| 级别 | 值 | 用途 |
|------|------|------|
| xs | 4px | 紧凑间距、图标与文字 |
| sm | 8px | 列表项内边距 |
| md | 16px | 区块内边距 |
| lg | 24px | 页面内边距 |
| xl | 40px | 大区块间距 |

## 3. 圆角

| 级别 | 值 | 用途 |
|------|------|------|
| 小 | `6px` | 按钮、标签、输入框 |
| 中 | `8px` | 卡片、面板、对话框 |
| 大 | `12px` | 大型容器 |

## 4. 阴影

| 级别 | 浅色 | 深色 | 用途 |
|------|------|------|------|
| sm | `0 1px 2px rgba(0,0,0,0.06)` | `0 1px 2px rgba(0,0,0,0.3)` | 轻微浮起 |
| md | `0 2px 8px rgba(0,0,0,0.08)` | `0 2px 8px rgba(0,0,0,0.4)` | 卡片浮起 |

## 5. 字体

| 元素 | 字体大小 | 字重 |
|------|---------|------|
| 页面标题 (h2) | 1.2em (≈19px) | 600 |
| 区块标题 (h3) | 15px | 600 |
| 正文 | 14px | 400 |
| 辅助文字 | 12-13px | 400 |
| 提示文字 | 11-12px | 400 |

字体栈: `系统默认 (无衬线)`

## 6. 主题模式

- `light` — 浅色主题
- `dark` — 深色主题（通过 `html.dark` 类切换）
- `system` — 跟随系统 `prefers-color-scheme`

持久化到 `localStorage` key `filetoolkit:theme`

## 7. 图标

- 应用图标: 各尺寸 png + ico + icns (已存在于 `src-tauri/icons/`)
- UI 图标: Element Plus Icons (`@element-plus/icons-vue`)

## 8. 组件样式覆盖

见 `src/styles/element-overrides.css` — 深色模式下微调 Element Plus 组件的背景、边框、文字颜色。
