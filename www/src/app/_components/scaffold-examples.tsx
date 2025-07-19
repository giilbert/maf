"use client";

import { Button } from "@/components/ui/button";
import { useState } from "react";

const SERVER_SCAFFOLDS = [
  {
    language: "Rust",
    files: ["src/lib.rs", "maf.toml", "Cargo.toml"],
  },
] as const;

export const ServerScaffoldExamples: React.FC<{
  codeBlocks: Record<string, React.ReactNode>;
}> = ({ codeBlocks }) => {
  const [selectedScaffold, setSelectedScaffold] = useState(0);
  const [selectedFile, setSelectedFile] = useState("src/lib.rs");

  const codeBlockKey = `${SERVER_SCAFFOLDS[selectedScaffold].language}:${selectedFile}`;
  const currentCodeBlock = codeBlocks[codeBlockKey];

  return (
    <div className="space-y-2">
      {SERVER_SCAFFOLDS.map((s, i) => (
        <Button
          size="sm"
          key={s.language}
          variant={i === selectedScaffold ? "secondary" : "ghost"}
          onClick={() => {
            setSelectedScaffold(SERVER_SCAFFOLDS.indexOf(s));
            setSelectedFile(s.files[0]);
          }}
        >
          {s.language}
        </Button>
      ))}

      <hr className="my-2" />

      <div className="flex gap-1">
        {SERVER_SCAFFOLDS[selectedScaffold].files.map((s) => (
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
        <pre className="bg-neutral-900 -mx-4 px-4 py-4 text-xs sm:text-sm md:text-base">
          {currentCodeBlock}
        </pre>
      ) : (
        <div>code block not found :/</div>
      )}
    </div>
  );
};

const CLIENT_SCAFFOLDS = [
  {
    language: "JavaScript/TypeScript",
    file: "client.ts",
  },
];

export const ClientScaffoldExamples: React.FC<{
  codeBlocks: Record<string, React.ReactNode>;
}> = ({ codeBlocks }) => {
  const [selectedScaffold, setSelectedScaffold] = useState(0);

  const { language, file } = CLIENT_SCAFFOLDS[selectedScaffold];
  const codeBlockKey = `${language}:${file}`;
  const currentCodeBlock = codeBlocks[codeBlockKey];

  return (
    <div className="space-y-2">
      <div className="mt-2 flex gap-1">
        {CLIENT_SCAFFOLDS.map((s, i) => (
          <Button
            size="sm"
            key={s.language}
            variant={i === selectedScaffold ? "secondary" : "ghost"}
            onClick={() => setSelectedScaffold(i)}
          >
            {s.language}
          </Button>
        ))}
      </div>

      {currentCodeBlock ? (
        <pre className="bg-neutral-900 -mx-4 px-4 py-4 text-xs sm:text-sm md:text-base">
          {currentCodeBlock}
        </pre>
      ) : (
        <div>code block not found :/</div>
      )}
    </div>
  );
};
