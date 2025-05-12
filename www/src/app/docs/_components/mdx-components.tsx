import { type MDXComponents } from "next-mdx-remote-client";
import { type BundledLanguage, codeToHtml } from "shiki";

export const mdxComponents: MDXComponents = {
  p: (props) => <p className="leading-relaxed" {...props}></p>,
  h1: (props) => <h1 className="text-4xl font-bold mb-2" {...props} />,
  h2: (props) => {
    return <h2 className="text-3xl font-bold" {...props} />;
  },
  h3: (props) => <h3 className="text-2xl font-semibold" {...props} />,
  h4: (props) => <h4 className="text-xl font-semibold" {...props} />,
  h5: (props) => <h5 className="text-lg font-semibold" {...props} />,
  code: (props) => {
    const { className } = props;
    const lang = className?.replace("language-", "") as BundledLanguage;

    return <CodeBlock lang={lang}>{props.children}</CodeBlock>;
  },
  pre: (props) => {
    return <pre className="border px-4 py-3">{props.children}</pre>;
  },
};

const CodeBlock: React.FC<{
  children: string;
  lang: BundledLanguage;
}> = async (props) => {
  const out = await codeToHtml(props.children, {
    lang: props.lang,
    theme: "github-dark-default",
    structure: "inline",
  });

  return <span dangerouslySetInnerHTML={{ __html: out }} />;
};
