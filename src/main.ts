import { createApp } from 'vue'
import { useDark, useToggle } from "@vueuse/core"
import ElementPlus from "element-plus";
import App from './App.vue'

import "element-plus/dist/index.css";
import "element-plus/theme-chalk/dark/css-vars.css";
import './style.css'

const app = createApp(App);
app.use(ElementPlus);
app.mount('#app');

export const isDark = useDark();
useToggle(isDark);
