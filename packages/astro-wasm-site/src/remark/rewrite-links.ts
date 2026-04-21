import { visit } from "unist-util-visit";
import type { Root } from "mdast";

interface RemarkRewriteLinksOptions {
  githubBlobBase: string;
}

export function remarkRewriteLinks(
  options: RemarkRewriteLinksOptions,
): (tree: Root) => void {
  const { githubBlobBase } = options;

  return (tree: Root) => {
    visit(tree, "link", (node) => {
      const url = node.url;

      if (/^(https?:\/\/|#)/.test(url)) {
        return;
      }

      const normalized = url.replace(/^\.\//, "");
      node.url = `${githubBlobBase}/${normalized}`;
    });
  };
}
