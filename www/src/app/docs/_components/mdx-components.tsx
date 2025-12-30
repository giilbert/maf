import { type MDXComponents } from "next-mdx-remote-client";
import Link from "next/link";
import { type BundledLanguage, codeToHtml } from "shiki";
import * as UiTabs from "@/components/ui/tabs";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { HeadingWrap } from "./heading-wrap";
import Image from "next/image";
import React from "react";
import {
  Dialog,
  DialogContent,
  DialogOverlay,
  DialogPortal,
  DialogTitle,
  DialogTrigger,
} from "@radix-ui/react-dialog";

export const mdxComponents: MDXComponents = {
  p: (props) => <p className="leading-relaxed" {...props}></p>,
  h1: (props) => (
    <HeadingWrap id={props.id}>
      <h1 className="text-4xl font-bold mb-2" {...props} />
    </HeadingWrap>
  ),
  h2: (props) => (
    <HeadingWrap id={props.id}>
      <h2 className="text-2xl font-bold pt-4" {...props} />
    </HeadingWrap>
  ),
  h3: (props) => (
    <HeadingWrap id={props.id}>
      <h3 className="text-xl font-semibold pt-2" {...props} />
    </HeadingWrap>
  ),
  h4: (props) => (
    <HeadingWrap id={props.id}>
      <h4 className="text-lg font-semibold pt-2" {...props} />
    </HeadingWrap>
  ),
  h5: (props) => (
    <HeadingWrap id={props.id}>
      <h5 className="text-base font-semibold pt-2" {...props} />
    </HeadingWrap>
  ),
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
  table: (props) => {
    return (
      <div className="overflow-x-auto">
        <table className="w-full border-collapse border">
          {props.children}
        </table>
      </div>
    );
  },
  th: (props) => {
    return (
      <th className="border px-3 py-2 bg-neutral-900 text-left text-sm whitespace-nowrap">
        {props.children}
      </th>
    );
  },
  td: (props) => {
    return (
      <td className="w-full border px-3 py-2 text-sm">{props.children}</td>
    );
  },
  Check: () => <p className="text-center w-full">✅</p>,
  X: () => <p className="text-center w-full">❌</p>,
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
  Collapsible: (props: { title: string; children: React.ReactNode }) => {
    return (
      <Accordion type="single" collapsible>
        <AccordionItem value="_">
          <AccordionTrigger className="bg-muted py-2 px-4">
            {props.title}
          </AccordionTrigger>
          <AccordionContent className="py-2 mt-2">
            {props.children}
          </AccordionContent>
        </AccordionItem>
      </Accordion>
    );
  },
  Image: (props: { src: string; aspectRatio: string; alt?: string }) => {
    const imgEl = (
      <Image
        src={props.src}
        alt={props.alt ?? ""}
        className="border w-full h-auto bg-neutral-800 relative before:absolute before:right-4 before:bottom-4"
        style={{
          aspectRatio: props.aspectRatio,
        }}
        width={9000}
        height={0}
      />
    );

    return (
      <Dialog>
        <DialogTrigger className="hover:brightness-75 cursor-pointer transition-all">
          {React.cloneElement(imgEl)}
        </DialogTrigger>
        <DialogPortal>
          <DialogOverlay className="fixed w-screen h-screen bg-black/80 top-0 left-0 cursor-pointer" />
          <DialogContent className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 max-w-[90vw] max-h-[90vh] w-full">
            <DialogTitle className="sr-only">
              {props.alt ?? "Image"}
            </DialogTitle>
            {React.cloneElement(imgEl, {
              className: "w-full h-auto",
            })}
          </DialogContent>
        </DialogPortal>
      </Dialog>
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
