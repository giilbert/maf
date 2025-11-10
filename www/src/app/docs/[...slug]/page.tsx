import { notFound } from "next/navigation";
import { getAllSlugs, getDocMeta, loadDocSource } from "../helpers/content";
import { renderMdx } from "../_components/mdx-renderer";
import { RenderPage } from "../_components/page";

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string[] }>;
}) {
  const slug = (await params).slug.join("/");

  const meta = await getDocMeta(slug);
  const source = await loadDocSource(slug);
  if (!meta || !source) return notFound();

  const { content, headings, defaultTabSelection } = await renderMdx({
    source,
  });

  return (
    <RenderPage
      content={content}
      headings={headings}
      meta={meta}
      defaultTabSelection={defaultTabSelection}
    />
  );
}

export async function generateStaticParams() {
  const slugs = await getAllSlugs();
  return slugs.map((slug) => ({ slug: slug.split("/") }));
}

export const dynamicParams = false;
