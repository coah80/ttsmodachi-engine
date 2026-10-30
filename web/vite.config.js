import { defineConfig } from "vite";

export default defineConfig({
  server: {
    proxy: { "/render": `http://127.0.0.1:${process.env.RUST_WORKER_PORT || 18082}` },
  },
});
