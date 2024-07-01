<template>
    <div>
        <div class="form-item">
            <el-upload drag multiple :before-upload="handleUpload">
                <el-icon class="el-icon--upload">
                    <DocumentAdd />
                </el-icon>
                <div class="el-upload__text">
                    拖拽图片到此处或<em>点此上传图片</em>
                </div>
            </el-upload>
        </div>

        <div class="form-item">
            <el-button @click="clearFiles" :icon="Delete">清空文件</el-button>
            <el-button @click="compressImagesWithInvokes" :icon="Download">打包下载</el-button>
        </div>

        <div class="form-item form-item-slider">
            <span class="form-label">压缩品质</span>
            <el-slider v-model="quality" :max="100" :min="0" :step="5"></el-slider>
        </div>

        <div class="form-item form-item-slider">
            <span class="form-label">线程数量</span>
            <el-slider v-model="batchSize" :max="16" :min="1" :step="1"></el-slider>
        </div>

        <div class="form-item">
            <p>待处理文件数量：{{ files.length }}</p>
            <p>总文件大小：{{ totalSizeMB }} MB</p>
        </div>

        <div class="form-item" v-if="loadInfo.isLoading">
            <!-- <progress :max="loadInfo.max" :value="loadInfo.current">{{ progress }}%</progress> -->
            <el-progress :percentage="progressPercentage" />
        </div>
    </div>
</template>


<script setup lang="ts">
import { ref, computed } from 'vue';
import { ElMessage } from "element-plus";
import { DocumentAdd, Delete, Download } from "@element-plus/icons-vue";
import { invoke } from "@tauri-apps/api/tauri";
import { createWriteStream } from 'streamsaver';
import { ZipWriter, BlobReader } from '@zip.js/zip.js';
import { listen } from '@tauri-apps/api/event';

interface imageTaskData { file_name: string, content: string, quality: number };
interface imageResultData { file_name: string, content: string };

const files = ref<File[]>([]);
const loadInfo = ref({ isLoading: false, max: 100, current: 0, startTime: new Date() });
const batchSize = ref(8);
const quality = ref(90);
const availableImgExt = ["png", "jpg", "jpeg", "gif", "webp"];  // 支持的图片格式

// 计算所有文件的总大小并转换为 MB
const totalSizeMB = computed(() => {
    const totalBytes = files.value.reduce((total, file) => total + file.size, 0);
    return (totalBytes / 1024 / 1024).toFixed(2);  // 转换为 MB，并保留两位小数
});

const progressPercentage = computed(() => {
    return (loadInfo.value.current * 100 / loadInfo.value.max).toFixed(2);
})

let existingFilenames = {} as { [key: string]: boolean };

function getUniqueFilename(originalName: string) {
    let baseName = originalName.replace(/\.[^/.]+$/, ""); // 移除扩展名
    let counter = 1;
    let newName = `${baseName}.webp`;

    while (existingFilenames[newName]) {
        newName = `${baseName}-${counter}.webp`;
        counter++;
    }

    existingFilenames[newName] = true; // 标记这个新文件名已经被使用
    return newName;
}

async function handleUpload(file: File) {
    let fileExt = file.name.split(".").pop()!.toLowerCase();
    if (availableImgExt.indexOf(fileExt) != -1) {
        files.value.push(file);
    } else {
        ElMessage({
            message: file.name + " 不是一个支持的文件。",
            type: "error",
        });
    }
    return false;
};

function clearFiles() {
    files.value = [];
    ElMessage({
        message: "已清空所有文件。",
        type: "info",
    });
}

async function compressImagesWithInvokes() {
    if (files.value.length === 0) {
        ElMessage({ message: "没有文件可以处理", type: "warning" });
        return;
    }

    loadInfo.value.isLoading = true;
    loadInfo.value.max = files.value.length;
    loadInfo.value.current = 0;
    loadInfo.value.startTime = new Date();

    existingFilenames = {};
    // 创建 ZIP 文件的写入流
    const fileStream = createWriteStream("compressed_images.zip");
    const writer = fileStream.getWriter();
    const zipWriter = new ZipWriter(new WritableStream({
        write(chunk) {
            return writer.write(chunk);
        },
        close() {
            writer.close();
        }
    }));

    let activeTasks = 0;
    let index = 0;  // 当前处理的文件索引
    const promises = <Promise<void>[]>[];  // 用于存储 Promise 的数组

    // 设置事件监听器
    const unlisten = await listen('imageProcessed', async event => {
        const { file_name, content } = event.payload as imageResultData;
        const unique_file_name = getUniqueFilename(file_name);
        const blob = base64toBlob(content, 'image/webp');
        await zipWriter.add(unique_file_name, new BlobReader(blob));
        loadInfo.value.current++;
        activeTasks--;
        processNextBatch();  // 处理下一批任务
    });

    // 定义任务处理函数
    async function processTask(file: File) {
        const fileData = await fileToBase64(file, quality.value);
        invoke("add_compress_image_task", { task: fileData });
    }

    // 定义批处理启动函数
    function processNextBatch() {
        while (activeTasks < batchSize.value && index < files.value.length) {
            promises.push(processTask(files.value[index]));
            activeTasks++;
            index++;
        }
        if (index >= files.value.length) {  // 所有任务都已启动
            Promise.all(promises).then(() => {
                zipWriter.close().then(() => {
                    unlisten();  // 取消监听
                    loadInfo.value.isLoading = false;
                    const end = new Date();
                    ElMessage({ message: `所有图片处理完成：${end.getTime() - loadInfo.value.startTime.getTime()} ms`, type: "success" })
                });
            }).catch(error => {
                ElMessage({ message: "处理图像时出错：" + error, type: "error" });
                loadInfo.value.isLoading = false;
            });
        }
    }

    // 启动第一批任务
    processNextBatch();
}

// 辅助函数：将 Base64 字符串转换为 Blob
function base64toBlob(base64: string, mimeType: string) {
    const byteCharacters = atob(base64);
    const byteArrays = [];

    for (let offset = 0; offset < byteCharacters.length; offset += 512) {
        const slice = byteCharacters.slice(offset, offset + 512);
        const byteNumbers = new Array(slice.length);
        for (let i = 0; i < slice.length; i++) {
            byteNumbers[i] = slice.charCodeAt(i);
        }
        const byteArray = new Uint8Array(byteNumbers);
        byteArrays.push(byteArray);
    }

    return new Blob(byteArrays, { type: mimeType });
}

// 辅助函数：将文件转换为字节数组的编码Base64
function fileToBase64(file: File, quality: number): Promise<imageTaskData> {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => {
            // 获取ArrayBuffer的结果，并创建Uint8Array
            const arrayBuffer = reader.result as ArrayBuffer;
            const uint8Array = new Uint8Array(arrayBuffer);

            // 将Uint8Array转换为Base64字符串
            const binaryString = Array.from(uint8Array).map(byte => String.fromCharCode(byte)).join('');
            const base64String = btoa(binaryString);

            // 解决返回文件名和Base64编码的内容
            resolve({ file_name: file.name, content: base64String, quality: quality });
        };
        reader.onerror = () => {
            reject(new Error('File reading failed'));
        };
        reader.readAsArrayBuffer(file);
    });
}
</script>


<style>
.form-item {
    margin: 10px;
}

.form-item-slider {
    display: flex;
    justify-content: center;
    align-items: center;
}

.form-item .form-label {
    font-size: 14px;
    color: var(--el-text-color-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.form-item .el-slider {
    margin-top: 0;
    margin-left: 12px;
    width: 50%;
}
</style>