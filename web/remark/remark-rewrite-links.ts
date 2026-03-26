import { visit } from "unist-util-visit";
import type { Root } from "mdast";
import { GITHUB_BASE } from "./consts";

export function remarkRewriteLinks() : (tree: Root) => void {
  return (tree: Root) => {
    visit(tree, "link", (node) => {
      const url = node.url;

      // Skip absolute URLs and anchors
      if (/^(https?:\/\/|#)/.test(url)) return;

      // Normalize ./foo to foo
      const normalized = url.replace(/^\.\//, "");
      node.url = `${GITHUB_BASE}/${normalized}`;
    });
  };
}
