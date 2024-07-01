<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import Compressor from "./components/Compressor.vue";
</script>

<template>
  <div class="app-container">
    <h1>WebP 压缩机</h1>
    <p class="text-gray-500 my-2 text-sm">
      并行压缩图片到 WebP 格式，打包作为 ZIP 文件下载
    </p>
    <Compressor />
    <div class="my-4 pt-4">
      <a class="inline-block text-sm text-gray-500" href="https://github.com/Exception0x0194/webp-compressor">GitHub</a>
    </div>
  </div>
</template>