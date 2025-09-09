import Link from "next/link";
import { Heading } from "../_lib/toc";

export const TableOfContents: React.FC<{
  headings: Heading[];
  hideTitle?: boolean;
}> = ({ headings, hideTitle }) => {
  return (
    <div className="sticky top-9 space-y-4">
      {!hideTitle && <h2 className="text-sm font-semibold">On this page</h2>}

      <ul className="flex flex-col gap-2">
        {headings.map((heading) => (
          <li
            key={heading.slug}
            className="text-sm text-muted-foreground hover:text-foreground hover:underline transition-colors"
            style={{
              marginLeft: `${(heading.level - 2) * 0.5}rem`,
            }}
          >
            <Link href={`#${heading.slug}`}>{heading.title}</Link>
          </li>
        ))}
      </ul>
    </div>
  );
};
