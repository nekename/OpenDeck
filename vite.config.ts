import { defineConfig } from "vite";

import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
	plugins: [sveltekit(), tailwindcss()],
	css: {
		transformer: "lightningcss",
	},
	clearScreen: false,
	server: {
		watch: {
			ignored: ["**/src-tauri/**", "**/target/**"],
		},
	},
});
