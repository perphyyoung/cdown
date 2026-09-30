import { createApp } from "vue";
import App from "@/app/App.vue";
import "@vuepic/vue-datepicker/dist/main.css";
import "@/style.css";

// 禁用 webview 默认右键菜单：右键功能只保留倒计时行的编辑/删除菜单与标题栏的自适应宽高
document.addEventListener("contextmenu", (e) => e.preventDefault());

createApp(App).mount("#app");
