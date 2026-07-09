<script setup lang="ts">
import { computed } from "vue";
import type { NodeType, ParamDef } from "../pipeline/types";

const props = defineProps<{
  nodeType: NodeType | null;
  params: Record<string, unknown>;
}>();

const emit = defineEmits<{
  (e: "update:param", key: string, value: unknown): void;
}>();

const hasParams = computed(() => (props.nodeType?.params?.length ?? 0) > 0);

function getParamType(p: ParamDef): string {
  const pt = p.paramType;
  if (typeof pt === "string") return pt;
  if (typeof pt === "object" && "slider" in pt) return "slider";
  return "string";
}

function getSliderConfig(p: ParamDef): { min: number; max: number; step: number } | null {
  const pt = p.paramType;
  if (typeof pt === "object" && "slider" in pt) return pt.slider;
  return null;
}

function getValue(key: string): unknown {
  return props.params[key] ?? null;
}

function setValue(key: string, value: unknown) {
  emit("update:param", key, value);
}
</script>

<template>
  <div class="param-form">
    <template v-if="nodeType">
      <h3 class="form-title">{{ nodeType.name }}</h3>
      <p class="form-desc">{{ nodeType.description }}</p>

      <el-divider />

      <div v-if="!hasParams" class="no-params">
        <el-icon :size="24" color="var(--el-text-color-placeholder)">
          <svg viewBox="0 0 24 24">
            <path
              d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"
              fill="currentColor"
            />
          </svg>
        </el-icon>
        <p>此节点无配置参数</p>
      </div>

      <div v-for="p in nodeType.params ?? []" :key="p.key" class="param-field">
        <label class="param-label">
          {{ p.label }}
          <span v-if="p.required" class="required-mark">*</span>
        </label>

        <!-- string -->
        <el-input
          v-if="getParamType(p) === 'string'"
          :model-value="String(getValue(p.key) ?? '')"
          :placeholder="String(p.default ?? '')"
          @update:model-value="setValue(p.key, $event)"
        />

        <!-- number -->
        <el-input-number
          v-else-if="getParamType(p) === 'number'"
          :model-value="Number(getValue(p.key) ?? undefined)"
          :placeholder="String(p.default ?? '')"
          controls-position="right"
          @update:model-value="setValue(p.key, $event)"
        />

        <!-- boolean -->
        <el-switch
          v-else-if="getParamType(p) === 'boolean'"
          :model-value="Boolean(getValue(p.key) ?? p.default ?? false)"
          @update:model-value="setValue(p.key, $event)"
        />

        <!-- select -->
        <el-select
          v-else-if="getParamType(p) === 'select'"
          :model-value="getValue(p.key) ?? p.default ?? ''"
          @update:model-value="setValue(p.key, $event)"
        >
          <el-option
            v-for="opt in p.options ?? []"
            :key="opt.value"
            :label="opt.label"
            :value="opt.value"
          />
        </el-select>

        <!-- slider -->
        <template v-else-if="getSliderConfig(p)">
          <el-slider
            :model-value="Number(getValue(p.key) ?? p.default ?? 0)"
            :min="getSliderConfig(p)!.min"
            :max="getSliderConfig(p)!.max"
            :step="getSliderConfig(p)!.step"
            show-input
            @update:model-value="setValue(p.key, $event)"
          />
        </template>

        <span v-if="p.help" class="param-help">{{ p.help }}</span>
      </div>
    </template>

    <template v-else>
      <div class="no-selection">
        <el-icon :size="32" color="var(--el-text-color-placeholder)">
          <svg viewBox="0 0 24 24">
            <path
              d="M11 7h2v2h-2zm0 4h2v6h-2zm1-9C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z"
              fill="currentColor"
            />
          </svg>
        </el-icon>
        <p>点击画布中的节点<br />查看和编辑参数</p>
      </div>
    </template>
  </div>
</template>

<style scoped>
.param-form {
  padding: 12px;
  height: 100%;
  overflow-y: auto;
}

.form-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}

.form-desc {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.no-params,
.no-selection {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 0;
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.param-field {
  margin-bottom: 14px;
}

.param-label {
  display: block;
  margin-bottom: 4px;
  font-size: 13px;
  font-weight: 500;
}

.required-mark {
  color: var(--el-color-danger);
}

.param-help {
  display: block;
  margin-top: 2px;
  font-size: 11px;
  color: var(--el-text-color-placeholder);
}
</style>
