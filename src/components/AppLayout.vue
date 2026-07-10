<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter, useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import { setLocale, availableLocales } from "../i18n";
import { useTheme, themeOptions } from "../composables/useTheme";
import {
  House,
  Picture,
  Document,
  EditPen,
  Search,
  Connection,
  FolderOpened,
  VideoCamera,
  Headset,
  Fold,
  Expand,
} from "@element-plus/icons-vue";

const SIDEBAR_KEY = "filetoolkit:sidebar-collapsed";

function loadSidebarState(): boolean {
  try {
    const raw = localStorage.getItem(SIDEBAR_KEY);
    return raw !== null ? raw === "true" : false;
  } catch {
    return false;
  }
}

function saveSidebarState(collapsed: boolean) {
  try {
    localStorage.setItem(SIDEBAR_KEY, String(collapsed));
  } catch {
    // localStorage 不可用时静默忽略(私密模式等)
  }
}

const router = useRouter();
const route = useRoute();
const { t } = useI18n();
const { themeMode, setTheme } = useTheme();
const isCollapsed = ref(loadSidebarState());

const navItems = computed(() => [
  { path: "/", label: t("nav.home"), icon: House },
  { path: "/image", label: t("nav.image"), icon: Picture },
  { path: "/pdf", label: t("nav.pdf"), icon: Document },
  { path: "/rename", label: t("nav.rename"), icon: EditPen },
  { path: "/dedup", label: t("nav.dedup"), icon: Search },
]);

const advancedItems = computed(() => [
  { path: "/video", label: t("nav.video"), icon: VideoCamera },
  { path: "/audio", label: t("nav.audio"), icon: Headset },
  { path: "/pipeline", label: t("nav.pipeline"), icon: Connection },
  { path: "/disk-usage", label: t("nav.diskUsage"), icon: FolderOpened },
]);

function handleSelect(path: string) {
  router.push(path);
}

function toggleSidebar() {
  isCollapsed.value = !isCollapsed.value;
  saveSidebarState(isCollapsed.value);
}
</script>

<template>
  <el-container class="app-layout">
    <!-- 侧边栏 -->
    <el-aside :width="isCollapsed ? '64px' : '200px'" class="app-aside">
      <div class="logo">
        <span v-if="!isCollapsed" class="logo-text">FileToolkit</span>
        <span v-else class="logo-text">FT</span>
      </div>
      <el-menu
        :default-active="route.path"
        :collapse="isCollapsed"
        :collapse-transition="false"
        class="app-menu"
        @select="handleSelect"
      >
        <el-menu-item v-for="item in navItems" :key="item.path" :index="item.path">
          <el-icon><component :is="item.icon" /></el-icon>
          <template #title>
            {{ item.label }}
          </template>
        </el-menu-item>
        <el-menu-item-group v-if="!isCollapsed" title="高级">
          <el-menu-item v-for="item in advancedItems" :key="item.path" :index="item.path">
            <el-icon><component :is="item.icon" /></el-icon>
            <template #title>
              {{ item.label }}
            </template>
          </el-menu-item>
        </el-menu-item-group>
        <template v-else>
          <el-menu-item v-for="item in advancedItems" :key="item.path" :index="item.path">
            <el-icon><component :is="item.icon" /></el-icon>
            <template #title>
              {{ item.label }}
            </template>
          </el-menu-item>
        </template>
      </el-menu>
    </el-aside>

    <!-- 主区域 -->
    <el-container>
      <el-header class="app-header">
        <el-icon class="collapse-btn" :size="20" @click="toggleSidebar">
          <component :is="isCollapsed ? Expand : Fold" />
        </el-icon>
        <div class="header-right">
          <el-select
            :model-value="themeMode"
            size="small"
            style="width: 100px"
            @change="(v) => setTheme(v)"
          >
            <el-option v-for="o in themeOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
          <el-select
            :model-value="$i18n.locale"
            size="small"
            style="width: 100px"
            @change="(v: string) => setLocale(v)"
          >
            <el-option v-for="loc in availableLocales" :key="loc.value" :label="loc.label" :value="loc.value" />
          </el-select>
        </div>
      </el-header>
      <el-main class="app-main">
        <router-view :key="route.path" />
      </el-main>
    </el-container>
  </el-container>
</template>

<style scoped>
.app-layout {
  height: 100vh;
}

.app-aside {
  border-right: 1px solid var(--el-border-color-light);
  overflow: hidden;
  transition: width 0.3s ease;
}

.logo {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 60px;
  border-bottom: 1px solid var(--el-border-color-light);
}

.logo-text {
  font-size: 18px;
  font-weight: 700;
  color: var(--el-color-primary);
  white-space: nowrap;
}

.app-menu {
  border-right: none;
}

.app-header {
  display: flex;
  align-items: center;
  height: 48px;
  border-bottom: 1px solid var(--el-border-color-light);
  padding: 0 16px;
}

.collapse-btn {
  cursor: pointer;
  color: var(--el-text-color-regular);
}

.collapse-btn:hover {
  color: var(--el-color-primary);
}

.header-right {
  margin-left: auto;
  display: flex;
  gap: 8px;
  align-items: center;
}

.app-main {
  flex: 1;
  min-height: 0;
  /* flex column 布局：让子元素可以通过 flex:1 撑满高度 */
  /* 同时 overflow-y:auto 让普通页面内容过长时正常滚动 */
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  padding: 0 !important; /* 覆盖 Element Plus el-main 默认 20px padding */
  background-color: var(--el-bg-color-page);
}
</style>
