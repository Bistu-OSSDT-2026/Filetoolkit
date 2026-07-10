<script setup lang="ts">
import { computed } from "vue";
import VChart from "vue-echarts";
import { use } from "echarts/core";
import { SunburstChart } from "echarts/charts";
import { TooltipComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { DirNode } from "../composables/useDiskScan";

use([SunburstChart, TooltipComponent, CanvasRenderer]);

const props = defineProps<{
  data: DirNode | null;
}>();

/** 将 DirNode 树转换为 ECharts 旭日图数据格式 */
function toSunburstData(node: DirNode): Record<string, unknown> {
  const item: Record<string, unknown> = {
    name: node.name,
    value: node.size,
    itemStyle: {},
  };

  if (node.children && node.children.length > 0) {
    // 仅目录有子节点
    const dirChildren = node.children.filter((c) => c.isDir);
    const fileChildren = node.children.filter((c) => !c.isDir);

    const allChildren: Record<string, unknown>[] = [];

    // 目录在前，文件在后
    for (const c of dirChildren) {
      allChildren.push(toSunburstData(c));
    }
    for (const c of fileChildren) {
      allChildren.push({
        name: c.name,
        value: c.size,
        itemStyle: { color: "#c6e2ff" }, // 文件用浅蓝色区分
      });
    }

    item.children = allChildren;
  }

  return item;
}

const option = computed(() => {
  if (!props.data) return {};

  const totalSize = props.data.size;

  return {
    tooltip: {
      formatter: (info: { name: string; value: number; treePathInfo?: { name: string }[] }) => {
        const path = (info.treePathInfo ?? []).map((p) => p.name).join(" / ");
        const size = formatBytes(info.value ?? 0);
        const pct = totalSize > 0 ? `(${(((info.value ?? 0) / totalSize) * 100).toFixed(1)}%)` : "";
        return `<b>${path}</b><br/>${size} ${pct}`;
      },
    },
    series: [
      {
        type: "sunburst",
        data: [toSunburstData(props.data)],
        radius: ["15%", "90%"],
        itemStyle: {
          borderRadius: 4,
          borderWidth: 2,
        },
        label: {
          rotate: "radial",
          fontSize: 11,
        },
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowColor: "rgba(0, 0, 0, 0.3)",
          },
        },
        levels: [
          {},
          { r0: "15%", r: "45%", label: { fontSize: 12 } },
          { r0: "45%", r: "65%", label: { fontSize: 10 } },
          { r0: "65%", r: "78%", label: { fontSize: 9 } },
          { r0: "78%", r: "90%", label: { fontSize: 8, position: "outside" } },
        ],
      },
    ],
  };
});

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
</script>

<template>
  <div class="sunburst-chart">
    <VChart v-if="data" :option="option" :autoresize="true" style="width: 100%; height: 100%" />
    <div v-else class="chart-empty">
      <p>选择目录开始扫描</p>
    </div>
  </div>
</template>

<style scoped>
.sunburst-chart {
  width: 100%;
  height: 100%;
  min-height: 400px;
}

.chart-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--el-text-color-secondary);
}
</style>
