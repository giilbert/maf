"use client";

import { Logo } from "@/components/logo";
import { cn } from "@/lib/cn";
import { useEffect, useState } from "react";
import { MenuIcon, XIcon } from "lucide-react";
import {
  Sheet,
  SheetContent,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import * as SheetPrimitive from "@radix-ui/react-dialog";
import { getDocsCategory } from "../helpers/content";
import { CategoriesRenderer } from "./side-nav";

export const MobileNav: React.FC<{
  categories: Awaited<ReturnType<typeof getDocsCategory>>;
}> = ({ categories }) => {
  const [isTop, setIsTop] = useState(true);

  useEffect(() => {
    const onScroll = () => {
      setIsTop(window.scrollY === 0);
    };

    window.addEventListener("scroll", onScroll);
    return () => {
      window.removeEventListener("scroll", onScroll);
    };
  }, []);

  return (
    <nav
      className={cn(
        "fixed sm:hidden top-0 left-0 w-screen p-2 bg-background flex items-center transition-all border-b border-transparent px-4",
        !isTop && "border-border/50"
      )}
    >
      <Logo size={24} className="mr-2" />
      <p className="font-bold">MAF</p>

      <Sheet>
        <SheetTrigger className="ml-auto p-1 hover:bg-muted rounded-sm transition-all cursor-pointer">
          <MenuIcon className="h-6 w-6" />
        </SheetTrigger>

        <SheetContent
          className="max-w-none sm:max-w-none w-full border-none p-2"
          showClose={false}
        >
          <div aria-describedby="mobile-nav-header" className="flex">
            <SheetTitle className="flex gap-2 items-center">
              <Logo size={24} className="m-2" />
              <span className="font-bold" id="mobile-nav-header">
                MAF Documentation
              </span>
            </SheetTitle>

            <SheetPrimitive.Close className="ml-auto w-max aspect-square p-1 hover:bg-muted rounded-sm transition-all cursor-pointer flex justify-center items-center">
              <XIcon className="w-4 h-4" />
            </SheetPrimitive.Close>
          </div>

          <div className="mx-2">
            <CategoriesRenderer categories={categories} />
          </div>
        </SheetContent>
      </Sheet>
    </nav>
  );
};
