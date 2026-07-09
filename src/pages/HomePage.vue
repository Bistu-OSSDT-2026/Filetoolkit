<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { Picture, Document, EditPen, Search } from "@element-plus/icons-vue";

const router = useRouter();
const { t } = useI18n();

const tools = computed(() => [
  { path: "/image", icon: Picture, title: t("home.imageCard"), description: t("home.imageDesc") },
  { path: "/pdf",    icon: Document, title: t("home.pdfCard"),    description: t("home.pdfDesc") },
  { path: "/rename", icon: EditPen,  title: t("home.renameCard"), description: t("home.renameDesc") },
  { path: "/dedup",  icon: Search,   title: t("home.dedupCard"),  description: t("home.dedupDesc") },
]);

function goTo(path: string) {
  router.push(path);
}
</script>

<template>
  <div class="home">
    <div class="hero">
      <h1>FileToolkit</h1>
      <p class="tagline">{{ t("home.welcome") }}</p>
      <p class="tagline-sub">{{ t("home.desc") }}</p>
    </div>

    <div class="tools-grid">
      <el-card
        v-for="tool in tools"
        :key="tool.path"
        class="tool-card"
        shadow="hover"
        @click="goTo(tool.path)"
      >
        <div class="tool-card-content">
          <el-icon :size="40" color="var(--el-color-primary)">
            <component :is="tool.icon" />
          </el-icon>
          <h3>{{ tool.title }}</h3>
          <p>{{ tool.description }}</p>
        </div>
      </el-card>
    </div>
  </div>
</template>

<style scoped>
.home {
  padding: 40px;
  max-width: 900px;
  margin: 0 auto;
}

.hero {
  text-align: center;
  margin-bottom: 48px;
}

h1 {
  margin: 0;
  font-size: 2.2em;
  color: var(--el-text-color-primary);
}

.tagline {
  color: var(--el-color-primary);
  font-weight: 500;
  margin-top: 0.5em;
  font-size: 1.1em;
}

.tagline-sub {
  color: var(--el-text-color-secondary);
  margin-top: 0.2em;
  font-size: 0.95em;
}

.tools-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 20px;
}

.tool-card {
  cursor: pointer;
  transition:
    transform 0.2s ease,
    border-color 0.2s ease;
}

.tool-card:hover {
  transform: translateY(-2px);
  border-color: var(--el-color-primary);
}

.tool-card-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 8px 0;
}

.tool-card-content h3 {
  margin: 12px 0 6px;
  font-size: 1.05em;
  color: var(--el-text-color-primary);
}

.tool-card-content p {
  margin: 0;
  font-size: 0.85em;
  color: var(--el-text-color-secondary);
}
</style>
