import { type MDXComponents } from "next-mdx-remote-client";
import Link from "next/link";
import { type BundledLanguage, codeToHtml } from "shiki";
import * as UiTabs from "@/components/ui/tabs";

export const mdxComponents: MDXComponents = {
  p: (props) => <p className="leading-relaxed" {...props}></p>,
  h1: (props) => <h1 className="text-4xl font-bold mb-2" {...props} />,
  h2: (props) => {
    return <h2 className="text-2xl font-bold pt-4" {...props} />;
  },
  h3: (props) => <h3 className="text-xl font-semibold pt-2" {...props} />,
  h4: (props) => <h4 className="text-lg font-semibold pt-2" {...props} />,
  h5: (props) => <h5 className="text-base font-semibold pt-2" {...props} />,
  code: (props) => {
    const { className } = props;
    const lang = className?.replace("language-", "") as BundledLanguage;

    return <CodeBlock lang={lang}>{props.children}</CodeBlock>;
  },
  pre: (props) => {
    return (
      <pre className="border px-4 py-3 overflow-x-auto text-xs md:text-sm w-full">
        {props.children}
      </pre>
    );
  },
  ul: (props) => {
    return <ul className="list-disc pl-5 space-y-1">{props.children}</ul>;
  },
  ol: (props) => {
    return <ol className="list-decimal pl-5 space-y-1">{props.children}</ol>;
  },
  a: (props) => {
    return (
      <Link href={props.href} className="underline underline-offset-3">
        {props.children}
      </Link>
    );
  },
  Tabs: UiTabs.Tabs,
  TabsList: UiTabs.TabsList,
  TabsTrigger: UiTabs.TabsTrigger,
  TabsContent: (props) => {
    return (
      <UiTabs.TabsContent value={props.value} className="space-y-5">
        {props.children}
      </UiTabs.TabsContent>
    );
  },
};

export const CodeBlock: React.FC<{
  children: string;
  lang: BundledLanguage;
}> = async (props) => {
  const out = await codeToHtml(props.children, {
    lang: props.lang,
    theme: "github-dark-default",
    structure: "inline",
  });

  return (
    <span className="font-mono" dangerouslySetInnerHTML={{ __html: out }} />
  );
};
