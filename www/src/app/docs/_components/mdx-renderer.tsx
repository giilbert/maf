import { evaluate } from "next-mdx-remote-client/rsc";
import remarkGfm from "remark-gfm";
import { mdxComponents } from "./mdx-components";
import { customHeadingId, extractToc, Heading } from "../_lib/toc";

interface RenderMdxOptions {
  source: string;
}

export const renderMdx = async (options: RenderMdxOptions) => {
  const { content, mod } = await evaluate({
    source: options.source,
    components: mdxComponents,
    options: {
      mdxOptions: {
        remarkPlugins: [remarkGfm, extractToc],
        rehypePlugins: [customHeadingId],
      },
    },
  });

  const headings: Heading[] =
    "headings" in mod ? JSON.parse(mod.headings as string) : [];
  const defaultTabSelection: Record<string, string> =
    "defaultTabSelection" in mod
      ? JSON.parse(mod.defaultTabSelection as string)
      : {};

  return {
    content,
    headings,
    defaultTabSelection,
  };
};
