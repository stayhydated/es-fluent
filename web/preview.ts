import { existsSync, statSync } from "node:fs";
import { extname, join, normalize } from "node:path";
import rootPackageJson from "../package.json" with { type: "json" };

const distDir = join(import.meta.dir, "dist");
const basePathName = readRootPackageName() ?? "es-fluent";
const basePath = `/${basePathName}`;
const host = process.env.HOST ?? "127.0.0.1";
const requestedPort = Number(process.env.PORT ?? "8081");

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

function readRootPackageName() {
  const rootPackageName = rootPackageJson.name.trim();

  return rootPackageName.length > 0 ? rootPackageName : null;
}

function isAddressInUse(error: unknown) {
  if (typeof error !== "object" || error === null) {
    return false;
  }

  const { code, message } = error as { code?: unknown; message?: unknown };
  return code === "EADDRINUSE" || (typeof message === "string" && message.includes("EADDRINUSE"));
}

function fetch(request: Request) {
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
}

function serve(port: number) {
  for (let candidatePort = port; candidatePort <= 65535; candidatePort += 1) {
    try {
      return Bun.serve({
        hostname: host,
        port: candidatePort,
        fetch,
      });
    } catch (error) {
      if (!isAddressInUse(error)) {
        throw error;
      }
    }
  }

  throw new Error(`No available port found at or above ${port}`);
}

const server = serve(requestedPort);

if (requestedPort !== 0 && server.port !== requestedPort) {
  console.warn(`Port ${requestedPort} is in use; using ${server.port} instead.`);
}

console.log(`Previewing SSG output at http://${server.hostname}:${server.port}${basePath}/`);
