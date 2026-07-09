<template>
  <div class="p-6 max-w-5xl mx-auto">
    <h1 class="text-2xl font-bold mb-6">C5 文件哈希校验工具</h1>

    <!-- 单文件计算哈希 -->
    <div class="border rounded p-5 mb-6">
      <h3 class="text-lg font-medium mb-3">1. 计算文件哈希值</h3>
      <div class="flex gap-3 mb-4">
        <input v-model="calcFilePath" placeholder="输入文件完整路径" class="flex-1 border px-3 py-2 rounded"/>
        <select v-model="calcAlgo" class="border px-3 py-2 rounded">
          <option value="md5">MD5</option>
          <option value="sha1">SHA1</option>
          <option value="sha256">SHA256</option>
          <option value="blake3">BLAKE3</option>
        </select>
        <button @click="calcHash" class="bg-blue-600 text-white px-5 py-2 rounded">开始计算</button>
      </div>
      <div v-if="calcProgress >= 0" class="mb-2">读取进度：{{ calcProgress.toFixed(2) }} %</div>
      <div v-if="calcResult" class="break-all bg-gray-100 p-3 rounded">文件哈希：{{ calcResult }}</div>
    </div>

    <!-- 单文件校验比对 -->
    <div class="border rounded p-5 mb-6">
      <h3 class="text-lg font-medium mb-3">2. 单文件哈希校验</h3>
      <input v-model="verifyFile" placeholder="文件路径" class="w-full border px-3 py-2 rounded mb-3"/>
      <input v-model="targetHash" placeholder="粘贴预期哈希值" class="w-full border px-3 py-2 rounded mb-3"/>
      <select v-model="verifyAlgo" class="border px-3 py-2 rounded mb-4">
        <option value="md5">MD5</option>
        <option value="sha1">SHA1</option>
        <option value="sha256">SHA256</option>
        <option value="blake3">BLAKE3</option>
      </select>
      <button @click="singleVerify" class="bg-green-600 text-white px-5 py-2 rounded">比对校验</button>
      <div v-if="verifyRes !== null" class="mt-4 text-xl font-medium" :class="verifyRes ? 'text-green-600' : 'text-red-600'">
        {{ verifyRes ? "✅ 文件哈希匹配，校验通过" : "❌ 哈希不一致，文件被修改" }}
      </div>
    </div>

    <!-- 批量校验模块 -->
    <div class="border rounded p-5">
      <h3 class="text-lg font-medium mb-3">3. 批量多文件校验</h3>
      <div v-for="(row, index) in batchList" :key="index" class="flex gap-2 mb-2 items-center">
        <input v-model="row.path" placeholder="文件路径" class="flex-1 border p-2 rounded"/>
        <input v-model="row.expected" placeholder="标准哈希" class="flex-1 border p-2 rounded"/>
        <select v-model="row.algorithm" class="border p-2 rounded">
          <option value="md5">md5</option>
          <option value="sha1">sha1</option>
          <option value="sha256">sha256</option>
          <option value="blake3">blake3</option>
        </select>
        <button @click="batchList.splice(index,1)" class="bg-red-400 text-white px-2 rounded">删除</button>
      </div>
      <div class="flex gap-3 my-4">
        <button @click="batchList.push({path:'',expected:'',algorithm:'sha256'})" class="px-4 py-2 border rounded">新增校验行</button>
        <button @click="batchCheckAll" class="bg-orange-500 text-white px-5 py-2 rounded">执行批量校验</button>
      </div>
      <div v-if="batchResult.length > 0" class="mt-4">
        <div v-for="(item, idx) in batchResult" :key="idx" class="my-1">
          <span>{{ item[0] }}</span>
          <span class="ml-3 font-medium" :class="item[1] ? 'text-green-600' : 'text-red-600'">
            {{ item[1] ? "✅ 通过" : "❌ 校验失败" }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// 单文件计算哈希
const calcFilePath = ref('')
const calcAlgo = ref('sha256')
const calcResult = ref('')
const calcProgress = ref(0)

// 单文件校验
const verifyFile = ref('')
const targetHash = ref('')
const verifyAlgo = ref('sha256')
const verifyRes = ref(null)

// 批量校验
const batchList = ref([{path:'',expected:'',algorithm:'sha256'}])
const batchResult = ref([])

// 监听进度事件
onMounted(async () => {
  await listen('checksum_progress', event => {
    calcProgress.value = event.payload[1]
  })
})

// 计算哈希
const calcHash = async () => {
  calcProgress.value = 0
  calcResult.value = ''
  try {
    const hashStr = await invoke('compute_checksum', {
      file: calcFilePath.value,
      algorithm: calcAlgo.value
    })
    calcResult.value = hashStr
  } catch (err) {
    alert(`计算失败：${err}`)
  }
}

// 单个文件比对
const singleVerify = async () => {
  verifyRes.value = null
  try {
    const match = await invoke('verify_checksum', {
      file: verifyFile.value,
      expected: targetHash.value,
      algorithm: verifyAlgo.value
    })
    verifyRes.value = match
  } catch (err) {
    alert(`校验失败：${err}`)
  }
}

// 批量校验
const batchCheckAll = async () => {
  batchResult.value = []
  const validList = batchList.value.filter(i => i.path && i.expected)
  if (validList.length === 0) return alert("请填写有效文件与哈希")
  try {
    const res = await invoke('batch_verify', { files: validList })
    batchResult.value = res
  } catch (err) {
    alert(`批量校验异常：${err}`)
  }
}
</script>