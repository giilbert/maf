import { evaluate } from "next-mdx-remote-client/rsc";
import { mdxComponents } from "./mdx-components";
import { customHeadingId, extractToc, Heading } from "../_lib/toc";
import { Suspense } from "react";

interface RenderMdxOptions {
  source: string;
}

export const renderMdx = async (options: RenderMdxOptions) => {
  const { content, mod } = await evaluate({
    source: options.source,
    components: mdxComponents,
    options: {
      mdxOptions: {
        remarkPlugins: [extractToc],
        rehypePlugins: [customHeadingId],
      },
    },
  });

  const headings: Heading[] =
    "headings" in mod ? JSON.parse(mod.headings as string) : [];

  return {
    content,
    headings,
  };
};

export const MdxContentWrapper: React.FC<{
  children: React.ReactNode;
}> = (props) => {
  return (
    <Suspense fallback={<></>}>
      <div className="flex flex-col gap-4" suppressHydrationWarning>
        {props.children}
      </div>
    </Suspense>
  );
};
