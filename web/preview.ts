import { existsSync, statSync } from "node:fs";
import { extname, join, normalize } from "node:path";

const distDir = join(import.meta.dir, "dist");
const basePath = "/es-fluent";
const host = process.env.HOST ?? "127.0.0.1";
const port = Number(process.env.PORT ?? "8081");

if (!existsSync(distDir)) {
  console.error(`Missing build output at ${distDir}`);
  console.error("Run `just web-build` first.");
  process.exit(1);
}

function safeJoin(relativePath: string) {
  const normalized = normalize(relativePath).replace(/^(\.\.(\/|\\|$))+/, "");
  return join(distDir, normalized);
}

function isFile(path: string) {
  return existsSync(path) && statSync(path).isFile();
}

function resolveFile(pathname: string) {
  const relativePath = pathname.replace(/^\/+/, "");
  const directPath = safeJoin(relativePath);

  if (isFile(directPath)) {
    return directPath;
  }

  if (!extname(relativePath)) {
    const indexPath = safeJoin(join(relativePath, "index.html"));
    if (isFile(indexPath)) {
      return indexPath;
    }
  }

  return null;
}

const server = Bun.serve({
  hostname: host,
  port,
  fetch(request) {
    const url = new URL(request.url);

    if (url.pathname === "/") {
      return Response.redirect(new URL(`${basePath}/`, url), 302);
    }

    if (!url.pathname.startsWith(basePath)) {
      return new Response("Not Found", { status: 404 });
    }

    const sitePath = url.pathname.slice(basePath.length) || "/";
    const resolvedPath = resolveFile(sitePath);
    if (resolvedPath) {
      return new Response(Bun.file(resolvedPath));
    }

    return new Response(Bun.file(join(distDir, "404.html")), { status: 404 });
  },
});

console.log(`Previewing SSG output at http://${server.hostname}:${server.port}${basePath}/`);
