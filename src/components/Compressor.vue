<template>
    <div style="display: flex;flex-direction: row;">
        <div style="width:400px">
            <div class="form-item-upload">
                <el-upload class="upload-files" drag @click="handleFileUpload" disabled style="margin-right: 10px">
                    <el-icon class="el-icon--upload">
                        <DocumentAdd />
                    </el-icon>
                    <div class="el-upload__text">
                        拖拽图片到窗口或<em>点此上传图片</em>
                    </div>
                </el-upload>
                <el-upload class="upload-folder" drag @click="handleFolderUpload" disabled style="margin-left: 10px">
                    <el-icon class="el-icon--upload">
                        <FolderAdd />
                    </el-icon>
                    <div class="el-upload__text">
                        <em>点此扫描文件夹内容</em>
                    </div>
                </el-upload>
            </div>

            <div class="form-item">
                <el-button @click="clearFiles" :icon="Delete">清空文件</el-button>
                <el-button @click="selectOutputFolder" :icon="FolderAdd">指定输出目录</el-button>
                <el-button @click="compressImagesWithInvokes" :icon="Download">压缩并保存</el-button>
            </div>

            <div class="form-item">
                <p>输出目录：{{ outputPath.length === 0 ? "尚未选择" : outputPath }}</p>
            </div>

            <div class="form-item form-item-slider">
                <span class="form-label">压缩品质</span>
                <el-slider v-model="quality" :max="100" :min="0" :step="5"></el-slider>
            </div>

            <div class="form-item">
                <el-checkbox v-model="keepDir"><span class="form-label">保持扫描的目录结构</span></el-checkbox>
            </div>

            <div class="form-item">
                <p>待处理文件数量：{{ files.length }}</p>
            </div>

            <div class="form-item" v-if="loadInfo.isLoading">
                <el-progress :percentage="progressPercentage" />
            </div>

            <div class="form-item" v-if="loadInfo.isLoading">
                <span class="file-name">处理文件：{{ compressedInfo.file_name }}</span>
                <br>
                <span>{{ compressedInfo.original_size.toFixed(2) }}→{{ compressedInfo.compressed_size.toFixed(2)
                    }}（{{ compressionRate }}）</span>
            </div>
        </div>
        <div style="width:500px; margin-left: 30px;">
            <el-table :data="files" stripe max-height="500">
                <el-table-column prop="path" label="输入路径" />
                <el-table-column label="操作" width="60">
                    <template #default="scope">
                        <el-button :icon="Delete" circle @click="removeFile(scope.$index)" />
                    </template>
                </el-table-column>
            </el-table>

        </div>
    </div>
</template>


<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { ElMessage, ElNotification } from "element-plus";
import { DocumentAdd, Delete, Download, FolderAdd } from "@element-plus/icons-vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { getCurrent } from "@tauri-apps/api/webview";

interface inputInfo { path: string, tree_path: string };

const files = ref<inputInfo[]>([]);
const loadInfo = ref({ isLoading: false, max: 100, current: 0, startTime: new Date() });
const compressedInfo = ref({ file_name: "", original_size: 0.0, compressed_size: 0.0 });
const keepDir = ref(false);
const quality = ref(90);
const outputPath = ref("");

const supportedExtensions = ['png', 'jpg', 'jpeg', 'gif', 'webp'];

const progressPercentage = computed(() => {
    return parseFloat((loadInfo.value.current * 100 / loadInfo.value.max).toFixed(2));
})

const compressionRate = computed(() => {
    if (compressedInfo.value.original_size === 0) {
        return "0.00 %";
    } else {
        return (compressedInfo.value.compressed_size * 100 / compressedInfo.value.original_size).toFixed(2) + ' %';
    }
})

function removeFile(index: number) {
    files.value.splice(index, 1);  // 从数组中删除指定索引的元素
}

async function handleFileUpload() {
    const res = await open({
        multiple: true,
        title: "Select Images",
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }]
    });

    if (res && res.length > 0) {
        const inputInfoArray = res.map(r => ({
            path: r.path,
            tree_path: ""
        }));

        files.value.push(...inputInfoArray); // 将这些对象推入files数组
        ElMessage({ message: `添加了 ${res.length} 份文件`, type: "success" });
    } else {
        ElMessage({ message: "未选择任何文件", type: "info" });
    }
    return false;
}


async function handleFolderUpload() {
    const dirPath = await open({
        directory: true,
        title: "Select Folder"
    });

    if (dirPath && dirPath.length !== 1) {
        const paths = await invoke('get_folder_file_paths', { dirPath: dirPath }) as inputInfo[];
        console.log(paths)
        files.value.push(...paths);
        ElMessage({ message: `添加了 ${paths.length} 份文件`, type: "success" });
    } else {
        ElMessage({ message: "未选择文件夹", type: "info" });
    }
    return false;
}

function clearFiles() {
    files.value = [];
    ElMessage({
        message: "已清空所有文件。",
        type: "info",
    });
}

async function selectOutputFolder() {
    const path = await open({
        directory: true,
        multiple: false,
        title: 'Select Output Directory'
    });
    if (typeof path !== "string") {
        ElMessage({ message: "没有选择输出目录", type: "info" });
        return;
    }
    outputPath.value = path;
    invoke('set_output_path', { outputPath: path })
}

async function compressImagesWithInvokes() {
    if (loadInfo.value.isLoading) {
        ElMessage({ message: "正在处理图片，请勿重复操作", type: "error" });
        return;
    }
    if (files.value.length === 0) {
        ElMessage({ message: "没有文件可以处理", type: "warning" });
        return;
    }
    if (outputPath.value.length === 0) {
        ElMessage({ message: "尚未选择输出目录", type: "warning" });
        return;
    }

    loadInfo.value.isLoading = true;
    loadInfo.value.max = files.value.length;
    loadInfo.value.current = 0;
    loadInfo.value.startTime = new Date();
    compressedInfo.value = { file_name: "", original_size: 0.0, compressed_size: 0.0 };

    let totalOriginalSize = 0;
    let totalCompressedSize = 0;

    const unlisten = await listen('singleTaskCompleted', async event => {
        const resultInfo = event.payload as { file_name: string, original_size: number, compressed_size: number };
        compressedInfo.value = resultInfo;
        totalOriginalSize += resultInfo.original_size;
        totalCompressedSize += resultInfo.compressed_size;
        loadInfo.value.current++;
        if (loadInfo.value.current === loadInfo.value.max) {
            const end = new Date();
            ElNotification({
                message: `全部文件处理完毕<br>用时：${end.getTime() - loadInfo.value.startTime.getTime()} ms<br>压缩率：${(totalCompressedSize * 100 / totalOriginalSize).toFixed(2)} %`, type: "success", dangerouslyUseHTMLString: true
            });
            loadInfo.value.isLoading = false;
            unlisten();
        }
    });

    invoke("add_compress_path_list", { inputDataList: files.value, quality: quality.value, outputPath: outputPath.value, keepDir: keepDir.value });
}
onMounted(async () => {
    const appWindow = getCurrent();
    await appWindow.onDragDropEvent((event) => {
        if (event.payload.type === 'dropped') {
            const filteredPaths = event.payload.paths.filter(path => {
                const extension = path.split('.').pop()!.toLowerCase();
                return supportedExtensions.includes(extension);
            });

            // 创建包含path和treePath的结构体数组
            const imageInputData = filteredPaths.map(path => ({
                path: path,
                tree_path: ""
            }));
            console.log('Dropped: ', imageInputData)

            files.value.push(...imageInputData);  // 推送结构体数组到files         
            ElMessage({ message: `添加了 ${filteredPaths.length} 份文件`, type: "success" });
        }
    });
    const path = await invoke('get_output_path') as string;
    outputPath.value = path;
})
</script>


<style scoped>
.form-item {
    margin-top: 10px;
    margin-bottom: 10px;
    white-space: nowrap;
}

.form-item-upload {
    display: inline-flex;
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

.file-name {
    display: block;
    max-width: 400px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
</style>