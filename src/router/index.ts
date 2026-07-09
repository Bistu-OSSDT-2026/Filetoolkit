import { createRouter, createWebHistory } from 'vue-router'
import HomePage from '../pages/HomePage.vue'
import RenamePage from '../pages/RenamePage.vue'
import DedupPage from '../pages/DedupPage.vue'
import PdfPage from '../pages/PdfPage.vue'
import ChecksumPage from '../pages/ChecksumPage.vue'

const routes = [
  {
    path: '/',
    name: '首页',
    component: HomePage
  },
  {
    path: '/rename',
    name: '批量重命名',
    component: RenamePage
  },
  {
    path: '/dedup',
    name: '重复文件清理',
    component: DedupPage
  },
  {
    path: '/pdf',
    name: 'PDF OCR文字提取',
    component: PdfPage
  },
  {
    path: '/checksum',
    name: '文件哈希校验',
    component: ChecksumPage
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

export default router