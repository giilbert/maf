import { notFound } from "next/navigation";
import { getAllSlugs, getDocMeta, loadDocSource } from "../helpers/content";
import { Suspense } from "react";
import { evaluate } from "next-mdx-remote-client/rsc";
import { mdxComponents } from "../_components/mdx-components";
import { customHeadingId, extractToc, Heading } from "../_lib/toc";

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;

  const meta = await getDocMeta(slug);
  const source = await loadDocSource(slug);
  if (!meta || !source) return notFound();

  const { content, mod } = await evaluate({
    source,
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
  console.log("headings", headings.map((h) => h.title).join(", "));

  return (
    <div className="space-y-2">
      <p className="text-muted-foreground">{meta.category}</p>

      <Suspense fallback={<div className="loading">Loading...</div>}>
        <div className="flex flex-col gap-3" suppressHydrationWarning>
          {content}
        </div>
      </Suspense>
    </div>
  );
}

export async function generateStaticParams() {
  const slugs = await getAllSlugs();
  return slugs.map((slug) => ({ slug }));
}

export const dynamicParams = false;
