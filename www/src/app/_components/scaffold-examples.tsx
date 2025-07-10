"use client";

import { Button } from "@/components/ui/button";
import { useState } from "react";

const SCAFFOLDS = [
  {
    language: "Rust",
    files: ["src/lib.rs", "maf.toml", "Cargo.toml"],
  },
] as const;

export const ScaffoldExamples: React.FC<{
  codeBlocks: Record<string, React.ReactNode>;
}> = ({ codeBlocks }) => {
  const [selectedScaffold, setSelectedScaffold] = useState(0);
  const [selectedFile, setSelectedFile] = useState("src/lib.rs");

  const codeBlockKey = `${SCAFFOLDS[selectedScaffold].language}:${selectedFile}`;
  console.log("codeBlockKey", codeBlockKey);
  const currentCodeBlock = codeBlocks[codeBlockKey];

  return (
    <div className="space-y-2">
      {SCAFFOLDS.map((s, i) => (
        <Button
          size="sm"
          key={s.language}
          variant={i === selectedScaffold ? "secondary" : "ghost"}
          onClick={() => {
            setSelectedScaffold(SCAFFOLDS.indexOf(s));
            setSelectedFile(s.files[0]);
          }}
        >
          {s.language}
        </Button>
      ))}

      <hr className="my-2" />

      <div className="flex gap-1">
        {SCAFFOLDS[selectedScaffold].files.map((s) => (
          <Button
            size="sm"
            key={s}
            variant={s === selectedFile ? "secondary" : "ghost"}
            onClick={() => setSelectedFile(s)}
          >
            {s}
          </Button>
        ))}
      </div>

      {currentCodeBlock ? (
        <pre className="bg-neutral-900 -mx-4 px-4 py-4">{currentCodeBlock}</pre>
      ) : (
        <div>code block not found :/</div>
      )}
    </div>
  );
};
