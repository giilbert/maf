"use client";

import {
  createContext,
  forwardRef,
  useContext,
  useEffect,
  useLayoutEffect,
} from "react";
import * as TabsPrimitive from "@radix-ui/react-tabs";
import { cn } from "@/lib/cn";

interface DocsTabsContextValue {
  tabSelections: Map<string, string>;
  scrollTops: Map<string, number>;
  setScrollTop: (id: string, scrollTop: number) => void;
  updateTabSelection: (id: string, selection: string) => void;
}

export const DocsTabsContext = createContext<DocsTabsContextValue | null>(null);

const Tabs = forwardRef<
  React.ComponentRef<typeof TabsPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Root> & {
    docId?: string;
  }
>(({ onValueChange, docId, ...props }, ref) => {
  const context = useContext(DocsTabsContext);

  if (context && !docId) {
    throw new Error(
      "Tabs must have an `docId` when used within <DocsTabsContext>. This is required to track tab selection."
    );
  }

  return (
    <TabsPrimitive.Root
      ref={ref}
      onValueChange={(value) => {
        if (docId && context) context.updateTabSelection(docId, value);
        onValueChange?.(value);
      }}
      {...props}
    />
  );
});
Tabs.displayName = TabsPrimitive.Root.displayName;

const TabsList = forwardRef<
  React.ComponentRef<typeof TabsPrimitive.List>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.List>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.List
    ref={ref}
    className={cn(
      "inline-flex h-10 items-center text-muted-foreground border-b w-full",
      className
    )}
    {...props}
  />
));
TabsList.displayName = TabsPrimitive.List.displayName;

const TabsTrigger = forwardRef<
  React.ComponentRef<typeof TabsPrimitive.Trigger>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Trigger>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.Trigger
    ref={ref}
    className={cn(
      "px-6 h-full border-b-2 border-transparent data-[state=active]:border-b-foreground data-[state=active]:text-foreground cursor-pointer hover:bg-background-300/50 box-content font-semibold",
      className
    )}
    {...props}
  />
));
TabsTrigger.displayName = TabsPrimitive.Trigger.displayName;

const TabsContent = forwardRef<
  React.ComponentRef<typeof TabsPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Content>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.Content
    ref={ref}
    className={cn("mt-5", className)}
    {...props}
  />
));
TabsContent.displayName = TabsPrimitive.Content.displayName;

export { Tabs, TabsList, TabsTrigger, TabsContent };
