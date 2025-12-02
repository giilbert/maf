"use client";

import { DocsTabsContext } from "@/components/ui/tabs";
import { useContext, useEffect, useRef } from "react";
import { Slot } from "@radix-ui/react-slot";

export const HeadingWrap: React.FC<{
  id: string;
  children: React.ReactNode;
}> = ({ id, children }) => {
  const docsTabs = useContext(DocsTabsContext);
  const ref = useRef<HTMLElement>(null);

  if (docsTabs) {
    const setScrollTop = docsTabs.setScrollTop;
    // docsTabs should be stable
    // eslint-disable-next-line react-hooks/rules-of-hooks
    useEffect(() => {
      if (!ref.current) return;

      const onResize = () => {
        setScrollTop(id, ref.current?.offsetTop || 0);
      };

      onResize();
      window.addEventListener("resize", onResize);
      return () => {
        window.removeEventListener("resize", onResize);
      };
    }, [setScrollTop, id]);
  }

  return <Slot ref={ref}>{children}</Slot>;
};
