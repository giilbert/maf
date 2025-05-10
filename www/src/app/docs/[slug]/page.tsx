import { notFound } from "next/navigation";
import { getAllSlugs, getDocMeta } from "../helpers/content";

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;

  const doc = await getDocMeta(slug);
  if (!doc) return notFound();

  const { default: Post } = await import(`@/content/${doc.slug}.mdx`);

  return <Post />;
}

export async function generateStaticParams() {
  const slugs = await getAllSlugs();
  return slugs.map((slug) => ({ slug }));
}

export const dynamicParams = false;
