export default async function Page({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const { default: Post } = await import(
    `@/content/${slug.replace("%2F", "/")}.mdx`
  );

  return <Post />;
}

const CONTENT_PATH = process.cwd() + "/src/content";

export async function generateStaticParams() {
  const { glob } = await import("glob");

  const files = await glob("**/*.{md,mdx}", { cwd: CONTENT_PATH });
  const slugs = files.map((file) => file.replace(/\.mdx?$/, ""));

  return slugs.map((slug) => ({
    slug: slug.replace(/\\/g, "/"),
  }));
}

export const dynamicParams = false;
